#![no_main]
#![no_std]

use core::sync::atomic::{AtomicU8, AtomicU32, Ordering};

#[macro_use]
mod macros;
mod sharp_lcd;
mod xiao_battery;

use defmt::{info, unwrap};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_nrf::gpio::{Input, Level, Output, OutputDrive, Pull};
use embassy_nrf::gpiote::{InputChannel, InputChannelPolarity};
use embassy_nrf::mode::Async;
use embassy_nrf::peripherals::{RNG, SPI3, USBD};
use embassy_nrf::saadc::Input as _;
use embassy_nrf::{bind_interrupts, rng, saadc, spim, usb};
use embassy_sync::channel::Channel;
use embassy_time::Timer;
use nrf_mpsl::Flash;
use nrf_sdc::mpsl::MultiprotocolServiceLayer;
use nrf_sdc::{self as sdc, mpsl};
use panic_probe as _;
use rmk::ble::build_ble_stack;
use rmk::config::StorageConfig;
use rmk::core_traits::Runnable;
use rmk::debounce::default_debouncer::DefaultDebouncer;
use rmk::event::{KeyboardEvent, publish_event_async};
use rmk::futures::future::join;
use rmk::input_device::battery::{BatteryProcessor, ChargingStateReader};
use rmk::input_device::rotary_encoder::Direction;
use rmk::matrix::Matrix;
use rmk::run_all;
use rmk::split::peripheral::run_rmk_split_peripheral;
use rmk::storage::new_storage_for_split_peripheral;
use rmk::watchdog::Nrf52Watchdog;
use rmk::{HostResources, RawMutex};
use static_cell::StaticCell;

use sharp_lcd::new_status_lcd;
use xiao_battery::{
    DIVIDER_MEASURED, DIVIDER_TOTAL, PeripheralBatterySnapshot, XiaoBatteryMonitor,
};

bind_interrupts!(struct Irqs {
    USBD => usb::InterruptHandler<USBD>;
    RNG => rng::InterruptHandler<RNG>;
    EGU0_SWI0 => nrf_sdc::mpsl::LowPrioInterruptHandler;
    CLOCK_POWER => nrf_sdc::mpsl::ClockInterruptHandler, usb::vbus_detect::InterruptHandler;
    RADIO => nrf_sdc::mpsl::HighPrioInterruptHandler;
    TIMER0 => nrf_sdc::mpsl::HighPrioInterruptHandler;
    RTC0 => nrf_sdc::mpsl::HighPrioInterruptHandler;
    SPIM3 => spim::InterruptHandler<SPI3>;
    SAADC => saadc::InterruptHandler;
});

#[embassy_executor::task]
async fn mpsl_task(mpsl: &'static MultiprotocolServiceLayer<'static>) -> ! {
    mpsl.run().await
}

const L2CAP_TXQ: u8 = 3;
const L2CAP_RXQ: u8 = 3;
const L2CAP_MTU: usize = 251;

fn build_sdc<'d, const N: usize>(
    peripherals: nrf_sdc::Peripherals<'d>,
    rng: &'d mut rng::Rng<Async>,
    mpsl: &'d MultiprotocolServiceLayer,
    memory: &'d mut sdc::Mem<N>,
) -> Result<nrf_sdc::SoftdeviceController<'d>, nrf_sdc::Error> {
    sdc::Builder::new()?
        .support_adv()
        .support_peripheral()
        .support_dle_peripheral()
        .support_phy_update_peripheral()
        .support_le_2m_phy()
        .peripheral_count(1)?
        .buffer_cfg(L2CAP_MTU as u16, L2CAP_MTU as u16, L2CAP_TXQ, L2CAP_RXQ)?
        .build(peripherals, rng, mpsl, memory)
}

fn ble_addr() -> [u8; 6] {
    let ficr = embassy_nrf::pac::FICR;
    let high = u64::from(ficr.deviceid(1).read());
    let addr = high << 32 | u64::from(ficr.deviceid(0).read());
    let addr = addr | 0x0000_c000_0000_0000;
    unwrap!(addr.to_le_bytes()[..6].try_into())
}

/// Rotary encoder reader for the left half.
///
/// Decode one event per mechanical detent from both encoder phases.
///
/// The measured encoder alternates stable `00` and `11` states at its detents.
/// Track the intermediate state so direction comes from the transition order,
/// rather than from a phase-B snapshot taken after a phase-A interrupt.
#[derive(Clone, Copy, PartialEq, Eq)]
enum EncoderPhase {
    A,
    B,
}

#[derive(Clone, Copy)]
struct EncoderEdge {
    phase: EncoderPhase,
    high: bool,
    sequence: u32,
}

const ENCODER_EDGE_QUEUE_SIZE: usize = 64;
static ENCODER_EDGE_CHANNEL: Channel<RawMutex, EncoderEdge, ENCODER_EDGE_QUEUE_SIZE> =
    Channel::new();
static ENCODER_PIN_STATE: AtomicU8 = AtomicU8::new(0);
static ENCODER_EDGE_SEQUENCE: AtomicU32 = AtomicU32::new(0);

fn capture_encoder_edge(phase: EncoderPhase, high: bool) -> EncoderEdge {
    let bit = match phase {
        EncoderPhase::A => 0b10,
        EncoderPhase::B => 0b01,
    };

    if high {
        ENCODER_PIN_STATE.fetch_or(bit, Ordering::AcqRel);
    } else {
        ENCODER_PIN_STATE.fetch_and(!bit, Ordering::AcqRel);
    }

    let sequence = ENCODER_EDGE_SEQUENCE
        .fetch_add(1, Ordering::AcqRel)
        .wrapping_add(1);

    EncoderEdge {
        phase,
        high,
        sequence,
    }
}

#[embassy_executor::task]
async fn encoder_a_edge_task(mut input: InputChannel<'static>) -> ! {
    loop {
        input.wait().await;
        ENCODER_EDGE_CHANNEL
            .send(capture_encoder_edge(EncoderPhase::A, input.pin().is_high()))
            .await;
    }
}

#[embassy_executor::task]
async fn encoder_b_edge_task(mut input: InputChannel<'static>) -> ! {
    loop {
        input.wait().await;
        ENCODER_EDGE_CHANNEL
            .send(capture_encoder_edge(EncoderPhase::B, input.pin().is_high()))
            .await;
    }
}

struct PendingDetent {
    target: u8,
    direction: Direction,
    sequence: u32,
}

struct LeftRotaryEncoder {
    /// AB is encoded as A in bit 1 and B in bit 0.
    state: u8,
    stable_detent: Option<u8>,
    first_phase: Option<EncoderPhase>,
}

impl LeftRotaryEncoder {
    fn new(a_high: bool, b_high: bool) -> Self {
        let state = Self::encode_state(a_high, b_high);
        Self {
            state,
            stable_detent: match state {
                0b00 | 0b11 => Some(state),
                _ => None,
            },
            first_phase: None,
        }
    }

    const fn encode_state(a_high: bool, b_high: bool) -> u8 {
        ((a_high as u8) << 1) | (b_high as u8)
    }

    fn update(&mut self, edge: EncoderEdge) -> Option<PendingDetent> {
        let bit = match edge.phase {
            EncoderPhase::A => 0b10,
            EncoderPhase::B => 0b01,
        };
        let next_state = if edge.high {
            self.state | bit
        } else {
            self.state & !bit
        };

        if next_state == self.state {
            return None;
        }

        let previous_state = self.state;
        self.state = next_state;

        match (previous_state, next_state) {
            // From 00 or 11, remember which phase moved first.
            (0b00, 0b01) | (0b11, 0b10) => {
                self.first_phase = Some(EncoderPhase::B);
                None
            }
            (0b00, 0b10) | (0b11, 0b01) => {
                self.first_phase = Some(EncoderPhase::A);
                None
            }

            // Reaching the opposite stable state creates a candidate. It is
            // emitted only after the candidate survives the stable-detent
            // check in run().
            (0b01, 0b11) => self.complete_detent(
                0b00,
                0b11,
                Some((EncoderPhase::B, Direction::CounterClockwise)),
                edge.sequence,
            ),
            (0b10, 0b11) => self.complete_detent(
                0b00,
                0b11,
                Some((EncoderPhase::A, Direction::Clockwise)),
                edge.sequence,
            ),
            (0b10, 0b00) => self.complete_detent(
                0b11,
                0b00,
                Some((EncoderPhase::B, Direction::CounterClockwise)),
                edge.sequence,
            ),
            (0b01, 0b00) => self.complete_detent(
                0b11,
                0b00,
                Some((EncoderPhase::A, Direction::Clockwise)),
                edge.sequence,
            ),

            // An impossible two-phase jump or an invalid intermediate step
            // is discarded and resynchronizes on the next stable detent.
            _ => {
                self.first_phase = None;
                None
            }
        }
    }

    fn complete_detent(
        &mut self,
        from: u8,
        target: u8,
        path: Option<(EncoderPhase, Direction)>,
        sequence: u32,
    ) -> Option<PendingDetent> {
        let pending = if self.stable_detent == Some(target) {
            None
        } else if self.stable_detent == Some(from)
            && path.is_some_and(|(phase, _)| self.first_phase == Some(phase))
        {
            path.map(|(_, direction)| PendingDetent {
                target,
                direction,
                sequence,
            })
        } else {
            None
        };

        self.first_phase = None;
        if pending.is_none() {
            // A stable state reached through an invalid or bouncing path is
            // still a useful synchronization point, but it must not emit an
            // event.
            self.stable_detent = Some(target);
        }
        pending
    }

    fn confirm_detent(&mut self, pending: PendingDetent) -> Option<Direction> {
        let sequence_unchanged = ENCODER_EDGE_SEQUENCE.load(Ordering::Acquire) == pending.sequence;
        let state_is_stable = ENCODER_PIN_STATE.load(Ordering::Acquire) == pending.target;

        if sequence_unchanged && state_is_stable {
            self.stable_detent = Some(pending.target);
            Some(pending.direction)
        } else {
            // Do not count a detent that was followed by another captured
            // edge during the debounce interval. Re-synchronize to the
            // latest captured stable state before consuming queued edges.
            let state = ENCODER_PIN_STATE.load(Ordering::Acquire);
            self.stable_detent = match state {
                0b00 | 0b11 => Some(state),
                _ => None,
            };
            self.first_phase = None;
            None
        }
    }
}

const ENCODER_EVENT_QUEUE_SIZE: usize = 64;
static ENCODER_DIRECTION_CHANNEL: Channel<RawMutex, Direction, ENCODER_EVENT_QUEUE_SIZE> =
    Channel::new();

#[embassy_executor::task]
async fn encoder_event_task() -> ! {
    loop {
        let direction = ENCODER_DIRECTION_CHANNEL.receive().await;
        publish_event_async(KeyboardEvent::rotary_encoder(0, direction, true)).await;
        // Keep the tap visible to the split transport while edge capture keeps
        // running independently in LeftRotaryEncoder::run.
        Timer::after_millis(5).await;
        publish_event_async(KeyboardEvent::rotary_encoder(0, direction, false)).await;
    }
}

impl Runnable for LeftRotaryEncoder {
    async fn run(&mut self) -> ! {
        loop {
            let edge = ENCODER_EDGE_CHANNEL.receive().await;
            if let Some(pending) = self.update(edge) {
                Timer::after_millis(2).await;
                if let Some(direction) = self.confirm_detent(pending) {
                    ENCODER_DIRECTION_CHANNEL.send(direction).await;
                }
            }
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Starting ReeL left (RMK peripheral)");

    let mut nrf_config = embassy_nrf::config::Config::default();
    nrf_config.lfclk_source = embassy_nrf::config::LfclkSource::ExternalXtal;
    nrf_config.dcdc.reg0_voltage = Some(embassy_nrf::config::Reg0Voltage::_3V3);
    nrf_config.dcdc.reg0 = true;
    nrf_config.dcdc.reg1 = true;
    let p = embassy_nrf::init(nrf_config);

    let mut lcd_spi_config = spim::Config::default();
    lcd_spi_config.frequency = spim::Frequency::M1;
    lcd_spi_config.mode = spim::MODE_0;
    lcd_spi_config.bit_order = spim::BitOrder::LsbFirst;
    let lcd_spi = spim::Spim::new_txonly(p.SPI3, Irqs, p.P1_00, p.P0_16, lcd_spi_config);
    let lcd_cs = Output::new(p.P1_10, Level::Low, OutputDrive::Standard);
    let (mut lcd, mut lcd_vcom) = new_status_lcd(lcd_spi, lcd_cs, false);

    let mut battery_monitor =
        XiaoBatteryMonitor::new(p.P0_31.degrade_saadc(), p.SAADC, p.P0_14).await;
    let mut battery_processor = BatteryProcessor::new(DIVIDER_MEASURED, DIVIDER_TOTAL);
    let mut charging_state_reader = ChargingStateReader::new(Input::new(p.P0_17, Pull::Up), true);
    let mut battery_snapshot = PeripheralBatterySnapshot::new();

    let mpsl_peripherals =
        mpsl::Peripherals::new(p.RTC0, p.TIMER0, p.TEMP, p.PPI_CH19, p.PPI_CH30, p.PPI_CH31);
    // XIAO nRF52840 has a 32.768 kHz crystal. Using it avoids LFRC
    // calibration work and its associated high-frequency-clock wakeups.
    // The RC-only fields must be zero when the XTAL source is selected.
    let lfclk_config = mpsl::raw::mpsl_clock_lfclk_cfg_t {
        source: mpsl::raw::MPSL_CLOCK_LF_SRC_XTAL as u8,
        rc_ctiv: 0,
        rc_temp_ctiv: 0,
        accuracy_ppm: mpsl::raw::MPSL_DEFAULT_CLOCK_ACCURACY_PPM as u16,
        skip_wait_lfclk_started: mpsl::raw::MPSL_DEFAULT_SKIP_WAIT_LFCLK_STARTED != 0,
    };
    static MPSL: StaticCell<MultiprotocolServiceLayer> = StaticCell::new();
    static SESSION_MEM: StaticCell<mpsl::SessionMem<1>> = StaticCell::new();
    let mpsl = MPSL.init(unwrap!(mpsl::MultiprotocolServiceLayer::with_timeslots(
        mpsl_peripherals,
        Irqs,
        lfclk_config,
        SESSION_MEM.init(mpsl::SessionMem::new()),
    )));
    spawner.spawn(mpsl_task(&*mpsl).unwrap());

    let sdc_peripherals = sdc::Peripherals::new(
        p.PPI_CH17, p.PPI_CH18, p.PPI_CH20, p.PPI_CH21, p.PPI_CH22, p.PPI_CH23, p.PPI_CH24,
        p.PPI_CH25, p.PPI_CH26, p.PPI_CH27, p.PPI_CH28, p.PPI_CH29,
    );
    let mut rng = rng::Rng::new(p.RNG, Irqs);
    let mut sdc_memory = sdc::Mem::<4696>::new();
    let sdc = unwrap!(build_sdc(sdc_peripherals, &mut rng, mpsl, &mut sdc_memory));
    let mut host_resources = HostResources::new();
    let stack = build_ble_stack(sdc, ble_addr(), &mut host_resources).await;

    let (row_pins, col_pins) = config_matrix_pins_nrf!(
        peripherals: p,
        input: [P0_02, P0_03, P0_28, P0_29],
        output: [P0_04, P0_05, P1_11, P1_12, P1_13, P0_09]
    );

    let storage_config = StorageConfig {
        start_addr: 0xA0000,
        num_sectors: 6,
        ..Default::default()
    };
    let flash = Flash::take(mpsl, p.NVMC);
    let mut storage = new_storage_for_split_peripheral(flash, storage_config).await;

    let debouncer = DefaultDebouncer::new();
    let mut matrix = Matrix::<_, _, _, 4, 6, true>::new(row_pins, col_pins, debouncer);
    let encoder_a = InputChannel::new(
        p.GPIOTE_CH0,
        p.P1_14,
        Pull::Up,
        InputChannelPolarity::Toggle,
    );
    let encoder_b = InputChannel::new(
        p.GPIOTE_CH1,
        p.P1_15,
        Pull::Up,
        InputChannelPolarity::Toggle,
    );
    let encoder_a_high = encoder_a.pin().is_high();
    let encoder_b_high = encoder_b.pin().is_high();
    ENCODER_PIN_STATE.store(
        LeftRotaryEncoder::encode_state(encoder_a_high, encoder_b_high),
        Ordering::Release,
    );
    ENCODER_EDGE_SEQUENCE.store(0, Ordering::Release);
    let mut encoder = LeftRotaryEncoder::new(encoder_a_high, encoder_b_high);
    spawner.spawn(encoder_a_edge_task(encoder_a).unwrap());
    spawner.spawn(encoder_b_edge_task(encoder_b).unwrap());
    spawner.spawn(encoder_event_task().unwrap());
    let mut watchdog = Nrf52Watchdog::default_runner(p.WDT);

    join(
        run_all!(
            matrix,
            encoder,
            storage,
            watchdog,
            battery_monitor,
            battery_processor,
            charging_state_reader,
            battery_snapshot,
            lcd,
            lcd_vcom
        ),
        run_rmk_split_peripheral(0, &stack),
    )
    .await;
}

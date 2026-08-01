use core::convert::Infallible;
use core::fmt::Write;

use defmt::unwrap;
use embassy_nrf::gpio::Output;
use embassy_nrf::spim::Spim;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::{FONT_6X10, FONT_10X20};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::text::{Baseline, Text};
use rmk::core_traits::Runnable;
use rmk::display::{DisplayDriver, DisplayProcessor, DisplayRenderer, RenderContext};
use rmk::heapless::String;
use rmk::types::battery::BatteryStatus;
use rmk::types::ble::BleState;
use static_cell::StaticCell;

const WIDTH: usize = 160;
const WIDTH_BYTES: usize = WIDTH / 8;
const HEIGHT: usize = 68;
const FRAMEBUFFER_SIZE: usize = WIDTH_BYTES * HEIGHT;
const WRITE_COMMAND: u8 = 0x01;
const VCOM_BIT: u8 = 0x02;
const CLEAR_COMMAND: u8 = 0x04;
const SCS_SETUP: Duration = Duration::from_micros(6);
const SCS_HOLD: Duration = Duration::from_micros(2);
const SCS_LOW: Duration = Duration::from_micros(6);
const VCOM_INTERVAL: Duration = Duration::from_millis(33);

static LCD_BUS: StaticCell<Mutex<ThreadModeRawMutex, SharpBus>> = StaticCell::new();

struct SharpBus {
    spi: Spim<'static>,
    cs: Output<'static>,
    vcom_high: bool,
}

impl SharpBus {
    fn new(spi: Spim<'static>, cs: Output<'static>) -> Self {
        Self {
            spi,
            cs,
            vcom_high: false,
        }
    }

    fn toggle_vcom(&mut self) -> u8 {
        self.vcom_high = !self.vcom_high;
        if self.vcom_high { VCOM_BIT } else { 0 }
    }

    async fn begin_transaction(&mut self) {
        Timer::after(SCS_LOW).await;
        self.cs.set_high();
        Timer::after(SCS_SETUP).await;
    }

    async fn end_transaction(&mut self) {
        Timer::after(SCS_HOLD).await;
        self.cs.set_low();
    }

    async fn clear(&mut self) {
        self.begin_transaction().await;
        let command = [CLEAR_COMMAND, 0];
        unwrap!(self.spi.write_from_ram(&command).await);
        self.end_transaction().await;
    }

    async fn write_frame(&mut self, framebuffer: &[u8; FRAMEBUFFER_SIZE]) {
        self.begin_transaction().await;

        let command = [WRITE_COMMAND | self.toggle_vcom()];
        unwrap!(self.spi.write_from_ram(&command).await);

        let mut line = [0_u8; WIDTH_BYTES + 2];
        for y in 0..HEIGHT {
            line[0] = (y + 1) as u8;
            line[1..=WIDTH_BYTES]
                .copy_from_slice(&framebuffer[y * WIDTH_BYTES..(y + 1) * WIDTH_BYTES]);
            unwrap!(self.spi.write_from_ram(&line).await);
        }

        let trailer = [0_u8];
        unwrap!(self.spi.write_from_ram(&trailer).await);
        self.end_transaction().await;
    }

    async fn invert_vcom(&mut self) {
        let command = [self.toggle_vcom(), 0];
        self.begin_transaction().await;
        unwrap!(self.spi.write_from_ram(&command).await);
        self.end_transaction().await;
    }
}

pub struct SharpDisplay {
    bus: &'static Mutex<ThreadModeRawMutex, SharpBus>,
    framebuffer: [u8; FRAMEBUFFER_SIZE],
    last_framebuffer: [u8; FRAMEBUFFER_SIZE],
    flushed_once: bool,
}

impl SharpDisplay {
    fn new(bus: &'static Mutex<ThreadModeRawMutex, SharpBus>) -> Self {
        Self {
            bus,
            // Sharp Memory LCD data is active-low: 1 is white and 0 is black.
            framebuffer: [0xff; FRAMEBUFFER_SIZE],
            last_framebuffer: [0xff; FRAMEBUFFER_SIZE],
            flushed_once: false,
        }
    }
}

impl OriginDimensions for SharpDisplay {
    fn size(&self) -> Size {
        Size::new(WIDTH as u32, HEIGHT as u32)
    }
}

impl DrawTarget for SharpDisplay {
    type Color = BinaryColor;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(point, color) in pixels {
            if point.x < 0 || point.y < 0 || point.x >= WIDTH as i32 || point.y >= HEIGHT as i32 {
                continue;
            }

            let x = point.x as usize;
            let index = point.y as usize * WIDTH_BYTES + x / 8;
            let mask = 1 << (x % 8);
            match color {
                BinaryColor::On => self.framebuffer[index] &= !mask,
                BinaryColor::Off => self.framebuffer[index] |= mask,
            }
        }
        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        let byte = match color {
            BinaryColor::On => 0x00,
            BinaryColor::Off => 0xff,
        };
        self.framebuffer.fill(byte);
        Ok(())
    }
}

impl DisplayDriver for SharpDisplay {
    async fn init(&mut self) {
        self.bus.lock().await.clear().await;
    }

    async fn flush(&mut self) {
        if self.flushed_once && self.framebuffer == self.last_framebuffer {
            return;
        }

        self.bus.lock().await.write_frame(&self.framebuffer).await;
        self.last_framebuffer.copy_from_slice(&self.framebuffer);
        self.flushed_once = true;
    }
}

pub struct SharpVcomRunner {
    bus: &'static Mutex<ThreadModeRawMutex, SharpBus>,
}

impl Runnable for SharpVcomRunner {
    async fn run(&mut self) -> ! {
        loop {
            Timer::after(VCOM_INTERVAL).await;
            self.bus.lock().await.invert_vcom().await;
        }
    }
}

pub struct ReelStatusRenderer {
    central: bool,
}

impl ReelStatusRenderer {
    fn new(central: bool) -> Self {
        Self { central }
    }
}

fn write_battery_level<const N: usize>(line: &mut String<N>, status: Option<BatteryStatus>) {
    match status {
        Some(BatteryStatus::Available {
            level: Some(level), ..
        }) => {
            let _ = write!(line, "{level}%");
        }
        _ => {
            let _ = line.push_str("--");
        }
    }
}

fn layer_name(layer: u8) -> &'static str {
    match layer {
        0 => "BASE",
        1 => "NUM/SYM",
        2 => "FN/NAV",
        3 => "AML",
        _ => "UNKNOWN",
    }
}

impl DisplayRenderer<BinaryColor> for ReelStatusRenderer {
    fn render<D: DrawTarget<Color = BinaryColor>>(&mut self, ctx: &RenderContext, display: &mut D) {
        let _ = display.clear(BinaryColor::Off);
        let status_style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
        let layer_style = MonoTextStyle::new(&FONT_10X20, BinaryColor::On);

        let mut battery_line: String<32> = String::new();
        if self.central {
            let _ = battery_line.push_str("BAT R:");
            write_battery_level(&mut battery_line, Some(ctx.battery.0));
            let _ = battery_line.push_str(" L:");
            write_battery_level(
                &mut battery_line,
                ctx.peripheral_batteries.first().map(|event| event.0),
            );
        } else {
            let _ = battery_line.push_str("BAT L:");
            write_battery_level(&mut battery_line, Some(ctx.battery.0));
        }
        let _ = Text::with_baseline(&battery_line, Point::new(0, 0), status_style, Baseline::Top)
            .draw(display);

        let ble_state = match ctx.ble_status.state {
            BleState::Advertising => "ADV",
            BleState::Connected => "CONNECTED",
            BleState::Inactive => "OFF",
        };
        let mut ble_line: String<24> = String::new();
        // Keep profile numbering consistent with vial.json (BT0, BT1, BT2).
        let _ = write!(ble_line, "BT{} {ble_state}", ctx.ble_status.profile);
        let _ = Text::with_baseline(&ble_line, Point::new(0, 11), status_style, Baseline::Top)
            .draw(display);

        let mut role_line: String<24> = String::new();
        if self.central {
            let connected = ctx.peripherals_connected.first().copied().unwrap_or(false);
            let _ = write!(
                role_line,
                "CENTRAL  PER:{}",
                if connected { "OK" } else { "--" }
            );
        } else {
            let _ = write!(
                role_line,
                "PERIPHERAL  CEN:{}",
                if ctx.central_connected { "OK" } else { "--" }
            );
        }
        let _ = Text::with_baseline(&role_line, Point::new(0, 22), status_style, Baseline::Top)
            .draw(display);

        let mut layer_line: String<16> = String::new();
        let _ = write!(layer_line, "L{} {}", ctx.layer, layer_name(ctx.layer));
        let x = (WIDTH as i32 - layer_line.len() as i32 * 10) / 2;
        let _ = Text::with_baseline(&layer_line, Point::new(x, 42), layer_style, Baseline::Top)
            .draw(display);
    }
}

pub fn new_status_lcd(
    spi: Spim<'static>,
    cs: Output<'static>,
    central: bool,
) -> (
    DisplayProcessor<SharpDisplay, ReelStatusRenderer>,
    SharpVcomRunner,
) {
    let bus = LCD_BUS.init(Mutex::new(SharpBus::new(spi, cs)));
    let display = SharpDisplay::new(bus);
    let processor = DisplayProcessor::with_renderer(display, ReelStatusRenderer::new(central))
        .with_min_render_interval(VCOM_INTERVAL);
    (processor, SharpVcomRunner { bus })
}

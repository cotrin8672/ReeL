use defmt::unwrap;
use embassy_nrf::gpio::Output;
use embassy_nrf::spim::Spim;
use embassy_time::{Duration, Timer};
use rmk::core_traits::Runnable;

const WIDTH_BYTES: usize = 160 / 8;
const HEIGHT: usize = 68;
const WRITE_COMMAND: u8 = 0x01;
const VCOM_BIT: u8 = 0x02;
const VCOM_INTERVAL: Duration = Duration::from_millis(33);
const LOGO: &[u8; WIDTH_BYTES * HEIGHT] = include_bytes!("reel_logo_160x68.raw");

/// Drives the BeeKeeb LS011B7DH03 and keeps its serial VCOM inversion alive.
pub struct SharpLcd {
    spi: Spim<'static>,
    cs: Output<'static>,
    vcom_high: bool,
}

impl SharpLcd {
    pub fn new(spi: Spim<'static>, cs: Output<'static>) -> Self {
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

    async fn write_logo(&mut self) {
        self.cs.set_high();

        let command = [WRITE_COMMAND | self.toggle_vcom()];
        unwrap!(self.spi.write_from_ram(&command).await);

        let mut line = [0_u8; WIDTH_BYTES + 2];
        for y in 0..HEIGHT {
            line[0] = (y + 1) as u8;
            line[1..=WIDTH_BYTES].copy_from_slice(&LOGO[y * WIDTH_BYTES..(y + 1) * WIDTH_BYTES]);
            unwrap!(self.spi.write_from_ram(&line).await);
        }

        let trailer = [0_u8];
        unwrap!(self.spi.write_from_ram(&trailer).await);
        self.cs.set_low();
    }

    async fn invert_vcom(&mut self) {
        let command = [self.toggle_vcom(), 0];
        self.cs.set_high();
        unwrap!(self.spi.write_from_ram(&command).await);
        self.cs.set_low();
    }
}

impl Runnable for SharpLcd {
    async fn run(&mut self) -> ! {
        self.write_logo().await;

        loop {
            Timer::after(VCOM_INTERVAL).await;
            self.invert_vcom().await;
        }
    }
}

use core::convert::Infallible;

use defmt::unwrap;
use embassy_nrf::gpio::Output;
use embassy_nrf::spim::Spim;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use rmk::core_traits::Runnable;
use rmk::display::{DisplayDriver, DisplayProcessor, DisplayRenderer, RenderContext};
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

pub struct ReelStatusRenderer;

impl ReelStatusRenderer {
    fn new(_central: bool) -> Self {
        Self
    }
}

impl DisplayRenderer<BinaryColor> for ReelStatusRenderer {
    fn render<D: DrawTarget<Color = BinaryColor>>(
        &mut self,
        _ctx: &RenderContext,
        display: &mut D,
    ) {
        let _ = display.clear(BinaryColor::Off);

        let outline = PrimitiveStyle::with_stroke(BinaryColor::On, 4);
        let fill = PrimitiveStyle::with_fill(BinaryColor::On);
        let _ = Rectangle::new(Point::new(34, 16), Size::new(80, 36))
            .into_styled(outline)
            .draw(display);
        let _ = Rectangle::new(Point::new(114, 27), Size::new(10, 14))
            .into_styled(fill)
            .draw(display);
        let _ = Rectangle::new(Point::new(42, 24), Size::new(32, 20))
            .into_styled(fill)
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

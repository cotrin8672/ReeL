use core::convert::Infallible;

use defmt::unwrap;
use embassy_nrf::gpio::Output;
use embassy_nrf::spim::Spim;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use rmk::core_traits::Runnable;
use rmk::display::{DisplayDriver, DisplayProcessor, DisplayRenderer, RenderContext};
use rmk::types::battery::BatteryStatus;
use static_cell::StaticCell;

const PANEL_WIDTH: usize = 160;
const PANEL_WIDTH_BYTES: usize = PANEL_WIDTH / 8;
const PANEL_HEIGHT: usize = 68;
const FRAMEBUFFER_SIZE: usize = PANEL_WIDTH_BYTES * PANEL_HEIGHT;

const LOGICAL_WIDTH: usize = 68;
const LOGICAL_HEIGHT: usize = 160;
const BASE_ROW_BYTES: usize = LOGICAL_WIDTH.div_ceil(8);
const BASE_UI: &[u8; BASE_ROW_BYTES * LOGICAL_HEIGHT] =
    include_bytes!("lcd_status_base_68x160.raw");

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
        unwrap!(self.spi.write_from_ram(&[CLEAR_COMMAND, 0]).await);
        self.end_transaction().await;
    }

    async fn write_frame(&mut self, framebuffer: &[u8; FRAMEBUFFER_SIZE]) {
        self.begin_transaction().await;

        let command = [WRITE_COMMAND | self.toggle_vcom()];
        unwrap!(self.spi.write_from_ram(&command).await);

        let mut line = [0_u8; PANEL_WIDTH_BYTES + 2];
        for y in 0..PANEL_HEIGHT {
            line[0] = (y + 1) as u8;
            line[1..=PANEL_WIDTH_BYTES]
                .copy_from_slice(&framebuffer[y * PANEL_WIDTH_BYTES..(y + 1) * PANEL_WIDTH_BYTES]);
            unwrap!(self.spi.write_from_ram(&line).await);
        }

        unwrap!(self.spi.write_from_ram(&[0]).await);
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
        Size::new(LOGICAL_WIDTH as u32, LOGICAL_HEIGHT as u32)
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
            if point.x < 0
                || point.y < 0
                || point.x >= LOGICAL_WIDTH as i32
                || point.y >= LOGICAL_HEIGHT as i32
            {
                continue;
            }

            // The physical 160x68 panel is mounted in portrait orientation.
            let physical_x = point.y as usize;
            let physical_y = LOGICAL_WIDTH - 1 - point.x as usize;
            let index = physical_y * PANEL_WIDTH_BYTES + physical_x / 8;
            let mask = 1 << (physical_x % 8);
            match color {
                BinaryColor::On => self.framebuffer[index] &= !mask,
                BinaryColor::Off => self.framebuffer[index] |= mask,
            }
        }
        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        self.framebuffer.fill(match color {
            BinaryColor::On => 0x00,
            BinaryColor::Off => 0xff,
        });
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

fn draw_pixel<D>(display: &mut D, x: i32, y: i32)
where
    D: DrawTarget<Color = BinaryColor>,
{
    let _ = display.draw_iter(core::iter::once(Pixel(Point::new(x, y), BinaryColor::On)));
}

fn draw_rectangle<D>(display: &mut D, x0: i32, y0: i32, x1: i32, y1: i32, filled: bool)
where
    D: DrawTarget<Color = BinaryColor>,
{
    for y in y0..=y1 {
        for x in x0..=x1 {
            if filled || x == x0 || x == x1 || y == y0 || y == y1 {
                draw_pixel(display, x, y);
            }
        }
    }
}

fn draw_base<D>(display: &mut D)
where
    D: DrawTarget<Color = BinaryColor>,
{
    for y in 0..LOGICAL_HEIGHT {
        for x in 0..LOGICAL_WIDTH {
            if BASE_UI[y * BASE_ROW_BYTES + x / 8] & (1 << (x % 8)) != 0 {
                draw_pixel(display, x as i32, y as i32);
            }
        }
    }
}

fn battery_level(status: BatteryStatus) -> Option<u8> {
    match status {
        BatteryStatus::Available {
            level: Some(level), ..
        } => Some(level.min(100)),
        _ => None,
    }
}

fn glyph(character: u8) -> (&'static [u8; 8], i32) {
    const DIGITS: [[u8; 8]; 10] = [
        [0x1c, 0x36, 0x22, 0x22, 0x22, 0x22, 0x36, 0x1c],
        [0x0c, 0x0a, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08],
        [0x0c, 0x12, 0x10, 0x10, 0x08, 0x04, 0x02, 0x1f],
        [0x0e, 0x11, 0x10, 0x0c, 0x10, 0x11, 0x11, 0x0e],
        [0x10, 0x18, 0x14, 0x14, 0x12, 0x3f, 0x10, 0x10],
        [0x1e, 0x01, 0x01, 0x0d, 0x13, 0x10, 0x11, 0x0e],
        [0x1c, 0x24, 0x22, 0x1e, 0x22, 0x22, 0x22, 0x1c],
        [0x1f, 0x10, 0x08, 0x08, 0x04, 0x04, 0x02, 0x02],
        [0x1c, 0x22, 0x22, 0x1c, 0x22, 0x22, 0x22, 0x1c],
        [0x1c, 0x22, 0x22, 0x22, 0x3c, 0x22, 0x12, 0x1c],
    ];
    const PERCENT: [u8; 8] = [0x0e, 0x2a, 0x2a, 0x1e, 0x30, 0x28, 0x24, 0x24];
    const HYPHEN: [u8; 8] = [0, 0, 0, 0, 0x03, 0, 0, 0];

    match character {
        b'0'..=b'9' => (&DIGITS[(character - b'0') as usize], 6),
        b'%' => (&PERCENT, 7),
        _ => (&HYPHEN, 3),
    }
}

fn draw_character<D>(display: &mut D, x: i32, y: i32, character: u8) -> i32
where
    D: DrawTarget<Color = BinaryColor>,
{
    let (rows, advance) = glyph(character);
    for (row, bits) in rows.iter().copied().enumerate() {
        for column in 0..6 {
            if bits & (1 << column) != 0 {
                draw_pixel(display, x + column, y + row as i32);
            }
        }
    }
    advance
}

fn draw_battery<D>(display: &mut D, level: Option<u8>)
where
    D: DrawTarget<Color = BinaryColor>,
{
    if let Some(level) = level {
        let fill_width = (13 * u16::from(level) + 50) / 100;
        if fill_width > 0 {
            draw_rectangle(display, 4, 6, 3 + i32::from(fill_width), 10, true);
        }
    }

    let mut x = 24;
    if let Some(level) = level {
        if level == 100 {
            x += draw_character(display, x, 4, b'1');
            x += draw_character(display, x, 4, b'0');
        } else if level >= 10 {
            x += draw_character(display, x, 4, b'0' + level / 10);
        }
        x += draw_character(display, x, 4, b'0' + level % 10);
    } else {
        x += draw_character(display, x, 4, b'-');
        x += draw_character(display, x, 4, b'-');
    }
    let _ = draw_character(display, x, 4, b'%');
}

fn draw_split_connection<D>(display: &mut D, connected: bool)
where
    D: DrawTarget<Color = BinaryColor>,
{
    for (x0, y0, x1, y1) in [(52, 9, 55, 12), (57, 6, 60, 12), (62, 3, 65, 12)] {
        draw_rectangle(display, x0, y0, x1, y1, connected);
    }
}

fn draw_active_layer<D>(display: &mut D, layer: u8)
where
    D: DrawTarget<Color = BinaryColor>,
{
    let y = 65 + i32::from(layer.min(3)) * 13;
    for (x, dy) in [(4, 0), (5, 1), (6, 1), (7, 2), (6, 3), (5, 3), (4, 4)] {
        draw_pixel(display, x, y + dy);
    }
}

fn draw_profiles<D>(display: &mut D, active_profile: u8)
where
    D: DrawTarget<Color = BinaryColor>,
{
    let active_profile = active_profile.min(4);
    for profile in 0..5 {
        let x = 23 + profile * 8;
        draw_rectangle(
            display,
            x,
            148,
            x + 4,
            152,
            profile == i32::from(active_profile),
        );
    }
}

impl DisplayRenderer<BinaryColor> for ReelStatusRenderer {
    fn render<D: DrawTarget<Color = BinaryColor>>(&mut self, ctx: &RenderContext, display: &mut D) {
        let _ = display.clear(BinaryColor::Off);
        draw_base(display);

        draw_battery(display, battery_level(ctx.battery.0));
        let split_connected = if self.central {
            ctx.peripherals_connected.first().copied().unwrap_or(false)
        } else {
            ctx.central_connected
        };
        draw_split_connection(display, split_connected);
        draw_active_layer(display, ctx.layer);
        draw_profiles(display, ctx.ble_status.profile);
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

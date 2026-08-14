from pathlib import Path

from PIL import Image, ImageDraw, ImageFont


WIDTH = 68
HEIGHT = 160
SCALE = 8

BLUETOOTH_GLYPH = (
    "...##..",
    "#..#.#.",
    ".#.#..#",
    "..##.#.",
    "...##..",
    "..##.#.",
    ".#.#..#",
    "#..#.#.",
    "...##..",
)


def draw_battery(draw: ImageDraw.ImageDraw, level: int) -> None:
    draw.rectangle((2, 4, 18, 12), outline=0)
    draw.rectangle((19, 7, 20, 9), fill=0)
    fill_width = round(13 * max(0, min(level, 100)) / 100)
    if fill_width:
        draw.rectangle((4, 6, 3 + fill_width, 10), fill=0)


def draw_bluetooth_icon(draw: ImageDraw.ImageDraw, x: int = 8, y: int = 146) -> None:
    for row_index, row in enumerate(BLUETOOTH_GLYPH):
        for column_index, pixel in enumerate(row):
            if pixel == "#":
                draw.point((x + column_index, y + row_index), fill=0)


def draw_current_profile(draw: ImageDraw.ImageDraw) -> None:
    draw_bluetooth_icon(draw)
    for profile in range(5):
        x = 23 + profile * 8
        draw.rectangle((x, 148, x + 4, 152), outline=0, fill=0 if profile == 0 else 1)


def draw_connection_indicator(draw: ImageDraw.ImageDraw) -> None:
    connected = True
    bars = ((52, 9, 55, 12), (57, 6, 60, 12), (62, 3, 65, 12))
    for box in bars:
        draw.rectangle(box, outline=0, fill=0 if connected else 1)


def draw_layer_list(draw: ImageDraw.ImageDraw, font: ImageFont.ImageFont) -> None:
    title_x = 4
    title_y = 46
    draw.text((title_x, title_y), "LAYERS", font=font, fill=0)
    draw.text((title_x + 1, title_y), "LAYERS", font=font, fill=0)
    draw.line((title_x, 55, 42, 55), fill=0)

    labels = ("BASE", "NUMBER", "SYMBOL", "MOUSE")
    for index, label in enumerate(labels):
        y = 63 + index * 13
        label_x = 13
        active = index == 0
        if active:
            draw.line((label_x - 9, y + 2, label_x - 6, y + 4), fill=0)
            draw.line((label_x - 6, y + 4, label_x - 9, y + 6), fill=0)
        draw.text((label_x, y - 1), label, font=font, fill=0)


def draw_link_status(draw: ImageDraw.ImageDraw) -> None:
    draw_current_profile(draw)


def draw_candidate_a(draw: ImageDraw.ImageDraw) -> None:
    draw.rounded_rectangle((6, 10, 26, 27), radius=3, outline=0)
    draw.rounded_rectangle((42, 10, 62, 27), radius=3, outline=0)
    for x, y in ((10, 14), (17, 14), (10, 21), (17, 21), (46, 14), (53, 14), (46, 21), (53, 21)):
        draw.rectangle((x, y, x + 2, y + 2), fill=0)
    draw.line((27, 18, 41, 18), fill=0, width=2)


def draw_candidate_b(draw: ImageDraw.ImageDraw) -> None:
    draw.rectangle((6, 11, 25, 26), outline=0)
    draw.rectangle((43, 11, 62, 26), outline=0)
    for x in (11, 16, 21, 48, 53, 58):
        draw.line((x, 13, x, 24), fill=0)
    draw.line((26, 18, 42, 18), fill=0, width=2)
    draw.rectangle((32, 16, 35, 20), outline=0)


def draw_candidate_c(draw: ImageDraw.ImageDraw) -> None:
    draw.line((8, 10, 6, 18, 8, 26), fill=0, width=2)
    draw.line((60, 10, 62, 18, 60, 26), fill=0, width=2)
    for x, y in ((12, 13), (17, 13), (12, 19), (17, 19), (51, 13), (56, 13), (51, 19), (56, 19)):
        draw.rectangle((x, y, x + 2, y + 2), fill=0)
    draw.line((22, 18, 46, 18), fill=0)
    draw.ellipse((31, 15, 36, 21), outline=0)


def draw_candidate_d(draw: ImageDraw.ImageDraw) -> None:
    for x in (7, 13, 19, 45, 51, 57):
        draw.rectangle((x, 12, x + 3, 24), outline=0)
    draw.line((27, 16, 41, 16), fill=0, width=2)
    draw.line((27, 20, 41, 20), fill=0, width=2)


def draw_unique_bridge(draw: ImageDraw.ImageDraw) -> None:
    draw.polygon(((4, 7), (23, 6), (27, 19), (8, 21)), outline=0)
    draw.polygon(((64, 7), (45, 6), (41, 19), (60, 21)), outline=0)
    for x, y in ((9, 10), (15, 9), (20, 9), (11, 15), (17, 14), (22, 14), (59, 10), (53, 9), (48, 9), (57, 15), (51, 14), (46, 14)):
        draw.rectangle((x, y, x + 2, y + 2), fill=0)
    draw.line((27, 14, 31, 8, 37, 8, 41, 14), fill=0, width=2)
    draw.line((27, 15, 41, 15), fill=0)


def draw_unique_zipper(draw: ImageDraw.ImageDraw) -> None:
    draw.line((8, 7, 25, 7, 28, 19, 10, 19), fill=0, width=2)
    draw.line((60, 7, 43, 7, 40, 19, 58, 19), fill=0, width=2)
    for y in (8, 12, 16):
        draw.line((24, y, 31, y + 2), fill=0, width=2)
        draw.line((44, y + 2, 37, y), fill=0, width=2)
    draw.line((31, 13, 37, 13), fill=0, width=2)


def draw_unique_magnet(draw: ImageDraw.ImageDraw) -> None:
    draw.line((7, 7, 7, 19, 23, 19, 23, 15), fill=0, width=2)
    draw.line((61, 7, 61, 19, 45, 19, 45, 15), fill=0, width=2)
    draw.rectangle((21, 12, 27, 16), fill=0)
    draw.rectangle((41, 12, 47, 16), fill=0)
    draw.line((29, 14, 39, 14), fill=0)
    draw.line((32, 10, 36, 18), fill=0)
    draw.line((36, 10, 32, 18), fill=0)


def draw_unique_radio(draw: ImageDraw.ImageDraw) -> None:
    draw.ellipse((5, 9, 15, 19), outline=0, fill=0)
    draw.ellipse((53, 9, 63, 19), outline=0, fill=0)
    draw.line((20, 9, 24, 13, 20, 17), fill=0, width=2)
    draw.line((48, 9, 44, 13, 48, 17), fill=0, width=2)
    draw.line((26, 6, 32, 13, 26, 20), fill=0)
    draw.line((42, 6, 36, 13, 42, 20), fill=0)


def draw_unique_chain(draw: ImageDraw.ImageDraw) -> None:
    draw.rounded_rectangle((5, 8, 31, 18), radius=4, outline=0, width=2)
    draw.rounded_rectangle((37, 8, 63, 18), radius=4, outline=0, width=2)
    draw.line((29, 9, 39, 17), fill=1, width=3)
    draw.line((29, 17, 39, 9), fill=0, width=2)
    draw.rectangle((11, 11, 13, 15), fill=0)
    draw.rectangle((55, 11, 57, 15), fill=0)


def draw_unique_plug(draw: ImageDraw.ImageDraw) -> None:
    draw.rectangle((6, 8, 22, 18), outline=0, width=2)
    draw.rectangle((46, 8, 62, 18), outline=0, width=2)
    draw.line((22, 11, 31, 11), fill=0, width=2)
    draw.line((22, 15, 31, 15), fill=0, width=2)
    draw.line((37, 11, 46, 11), fill=0, width=2)
    draw.line((37, 15, 46, 15), fill=0, width=2)
    draw.line((31, 13, 37, 13), fill=0, width=2)
    draw.rectangle((51, 11, 53, 15), fill=0)


def write_candidate_outputs(output_dir: Path) -> None:
    font = ImageFont.load_default()
    candidates = (
        ("A", draw_unique_bridge),
        ("B", draw_unique_zipper),
        ("C", draw_unique_magnet),
        ("D", draw_unique_radio),
        ("E", draw_unique_chain),
        ("F", draw_unique_plug),
    )
    preview = Image.new("1", (68, len(candidates) * 26), 1)
    for index, (label, renderer) in enumerate(candidates):
        strip = Image.new("1", (68, 26), 1)
        strip_draw = ImageDraw.Draw(strip)
        strip_draw.text((1, 1), label, font=font, fill=0)
        renderer(strip_draw)
        strip.save(output_dir / f"lcd-split-indicator-unique-{label.lower()}-68x26.png", optimize=False)
        preview.paste(strip, (0, index * 26))
    preview.resize((68 * SCALE, preview.height * SCALE), Image.Resampling.NEAREST).save(
        output_dir / "lcd-split-indicator-candidates-preview-8x.png", optimize=False
    )


def write_firmware_base(image: Image.Image, repository_root: Path) -> None:
    base = image.copy()
    draw = ImageDraw.Draw(base)

    # Dynamic regions are rendered from the live RMK status in sharp_lcd.rs.
    draw.rectangle((4, 6, 16, 10), fill=1)
    draw.rectangle((24, 2, 47, 12), fill=1)
    draw.rectangle((52, 3, 65, 12), fill=1)
    draw.rectangle((3, 62, 8, 110), fill=1)
    draw.rectangle((23, 148, 59, 152), fill=1)

    row_bytes = (WIDTH + 7) // 8
    packed = bytearray(row_bytes * HEIGHT)
    for y in range(HEIGHT):
        for x in range(WIDTH):
            if base.getpixel((x, y)) == 0:
                packed[y * row_bytes + x // 8] |= 1 << (x % 8)

    (repository_root / "src" / "lcd_status_base_68x160.raw").write_bytes(packed)


def main() -> None:
    repository_root = Path(__file__).resolve().parents[1]
    output_dir = repository_root / "docs" / "lcd"
    output_dir.mkdir(parents=True, exist_ok=True)

    image = Image.new("1", (WIDTH, HEIGHT), 1)
    draw = ImageDraw.Draw(image)
    font = ImageFont.load_default()

    draw_battery(draw, 78)
    draw.text((24, 2), "78%", font=font, fill=0)
    draw_connection_indicator(draw)
    draw.line((0, 18, WIDTH - 1, 18), fill=0)
    draw_layer_list(draw, font)
    draw.line((0, 141, WIDTH - 1, 141), fill=0)
    draw_link_status(draw)

    exact_path = output_dir / "lcd-status-dashboard-68x160.png"
    preview_path = output_dir / "lcd-status-dashboard-portrait-preview-8x.png"
    image.save(exact_path, optimize=False)
    image.resize((WIDTH * SCALE, HEIGHT * SCALE), Image.Resampling.NEAREST).save(
        preview_path, optimize=False
    )
    write_firmware_base(image, repository_root)
    write_candidate_outputs(output_dir)


if __name__ == "__main__":
    main()

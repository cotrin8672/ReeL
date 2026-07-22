# ReeL ZMK firmware

ReeL uses two Seeed XIAO nRF52840 controllers. The right half is the ZMK
central and contains the PMW3610 trackball. The left half is the BLE
peripheral and contains the rotary encoder. Each half has an LS011B7DH03
160x68 memory LCD.

## Build outputs

The `Build ZMK firmware` GitHub Actions workflow builds:

- `reel_left.uf2`
- `reel_right.uf2`

Run the workflow manually from the Actions tab, or push a change under
`boards/`, `config/`, `zephyr/`, `build.yaml`, or the workflow itself. The
merged `reel-firmware` artifact contains both UF2 files.

The ZMK source and reusable workflow are pinned to commit
`904c9aec8822d79149d42c8a9a77e8828eb08f5a` so that builds stay reproducible.

## Verified build

Both shield targets were built successfully against the pinned ZMK source with
Zephyr 4.1.0 and Zephyr SDK 0.17.0. Local verification produced the following
ignored artifacts:

- `build/firmware/reel_left.uf2`
- `build/firmware/reel_right.uf2`

Both generated configurations enable `CONFIG_ZMK_DISPLAY`, `CONFIG_LS0XX`,
one-bit LVGL color, and the built-in ZMK status screen. The generated
right-half configuration also enables `CONFIG_ZMK_SPLIT_ROLE_CENTRAL`,
`CONFIG_ZMK_POINTING`, `CONFIG_INPUT_PMW3610`, and `CONFIG_SPI`. The generated
left-half configuration also enables `CONFIG_EC11`.

## First flash

1. Put each XIAO into its UF2 bootloader by double-tapping reset.
2. Copy `reel_left.uf2` to the left controller.
3. Copy `reel_right.uf2` to the right controller.
4. Power both halves on. Pair the host with `ReeL`, advertised by the right
   central half.

If split bonding becomes inconsistent while iterating, build and flash the
standard `settings_reset` shield for `xiao_ble//zmk` to both halves before
flashing ReeL again.

## Bring-up assumptions

- The matrix diode direction is `col2row`.
- The first three matrix rows are transformed into physical left-to-right
  order on both halves. The key bindings remain provisional until the final
  keymap is specified.
- The two tact switches on the encoder shaft extension are left row 3,
  columns 0 and 1. Their current left/right click bindings are placeholders.
- The encoder is initially configured for 20 steps and 20 triggers per
  rotation. Adjust both values after checking the actual encoder behavior.
- The PMW3610 starts at 1200 CPI with no axis swap or inversion. Tune the
  devicetree properties after checking the installed sensor orientation.
- PMW3610 SDIO uses XIAO D10 as a half-duplex SPI data line. D8 is SCLK, D7
  is chip select, and D9 is MOTION.
- Each LS011B7DH03 uses P0.16 for SI, P1.00 for SCK, and P1.10 for the
  active-high chip select. The display uses SPIM3 and serial VCOM inversion;
  the right-side trackball remains on SPIM2.

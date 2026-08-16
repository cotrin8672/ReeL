# ReeL firmware

This branch carries the RMK port for the two Seeed XIAO nRF52840 controllers.
The right half is the BLE/USB central and contains the PMW3610 trackball. The
left half is the BLE peripheral and contains the rotary encoder. The Sharp
memory LCDs are intentionally outside the current RMK port.

## Implemented setup

- 4x11 unified matrix with the same 41 physical positions and four layers as
  the previous firmware
- BLE split with the right half at columns 6..10 and the left half at columns
  0..5
- PMW3610 at 1200 CPI and 125 Hz on the right half
- Calibrated fixed-point direction transform with input-length normalization and
  retained per-axis remainder:

  ```text
  transformed = M · raw
  output = transformed · |raw| / |transformed|
  ```

  The default direction matrix is:

  ```text
  M = [-0.265,  1.142]
      [-0.831,  0.562]
  ```

- Auto Mouse Layer 3 with a five-second timeout
- Mouse buttons 1/2 on the J/K positions while the mouse layer is active
- Left encoder mapped to vertical scrolling on every layer
- BLE/USB Vial support with flash-backed keymap storage

## Vial layout source

`vial.json` is regenerated from the left and right KiCad PCB matrix netlists
and switch positions. The generator follows each switch's `ColN` net to its
diode and then follows that diode to `RowN`; the right half is offset to
unified matrix columns 6..10. It uses the PCB positions to preserve the
mirrored row and thumb-key order in Vial.
Run it from the repository root after a matrix or PCB change:

```powershell
python tools/generate_vial_json.py
```

The script fails on ambiguous matrix wiring or an unsupported physical
arrangement, so a new or removed switch cannot silently disappear from Vial.
Use `--check` to verify that the checked-in JSON is current without writing it.

## Browser trackball calibration

Open `tools/trackball-profiler.html` in Chrome or Edge and select
**ワイヤレス接続**. The profiler uses RMK's existing vendor-defined Vial HID
collection, so the same page works through either USB or the bonded BLE
keyboard connection.

The last 28 bytes of RMK's macro buffer are reserved for a versioned `RLC1`
calibration record. The record contains the four fixed-point matrix
coefficients and an FNV-1a checksum. A valid write is persisted by RMK's normal
flash-backed Vial path and is applied to the live pointing pipeline within
25 ms; reflashing or rebooting is not required.

The profiler reads the current matrix on connection. With automatic apply
enabled, it writes the fitted replacement matrix after measurement settles.
The firmware rejects corrupt, near-singular, and excessively large matrices
and keeps the last valid value.

## Build outputs

The `Build RMK firmware` GitHub Actions workflow produces:

- `reel_right.uf2` for the right central
- `reel_left.uf2` for the left peripheral

For a local build, install stable Rust with the `thumbv7em-none-eabihf` target,
`llvm-tools`, `cargo-binutils`, and `cargo-hex-to-uf2`, then run:

```powershell
cargo build --release --locked --bins
cargo objcopy --release --locked --bin reel_right -- -O ihex reel_right.hex
cargo hex-to-uf2 --input-path reel_right.hex --output-path reel_right.uf2 --family nrf52840
cargo objcopy --release --locked --bin reel_left -- -O ihex reel_left.hex
cargo hex-to-uf2 --input-path reel_left.hex --output-path reel_left.uf2 --family nrf52840
```

## First flash

1. Put each XIAO into its UF2 bootloader by double-tapping reset.
2. Flash `reel_left.uf2` to the left controller.
3. Flash `reel_right.uf2` to the right controller.
4. Power both halves on and pair the host with `ReeL`.

RMK uses its own BLE split bonding and storage format. Flash both halves from
the same build when the split protocol changes. The first on-device validation
should cover all 41 switches, the encoder direction, trackball direction and
gain, five-second AML release, BLE split reconnection, and Vial persistence.

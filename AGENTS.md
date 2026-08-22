@C:\Users\gummy\.codex\RTK.md

# ReeL project notes for agents

## RMK firmware completion

- For RMK firmware changes, do not report completion from local checks alone. Push the working branch, wait for the `Build RMK firmware` GitHub Actions workflow to succeed, and confirm that it produced the `reel-rmk-firmware` artifact before merging or cherry-picking the change into `main`.
- Treat GitHub Actions as build and artifact evidence only. Physical cursor, encoder, display, battery, BLE, or split behavior still requires an explicit device test.

## Left PCB DRC intent

When checking DRC for `left/reel-left.kicad_pcb`, do not treat every KiCad DRC error as a real blocker. This board has several intentional mechanical/manufacturing features.

- The MCU is a Seeed XIAO BLE Plus. The MCU area intentionally uses narrow-pitch pads and tight geometry. DRC findings around U1 annular width or tight XIAO-related geometry should be judged against the actual XIAO BLE Plus footprint and manufacturer capabilities, not treated as automatically fatal.
- U1 has rectangular `Edge.Cuts` openings near pads 15/16/26/27. These are intentional cutouts for soldering the XIAO BLE Plus back-side pads. KiCad may report `copper_edge_clearance` at `0.0000 mm` around these openings; classify that as intentional if the opening shape is still the desired solder-access cutout.
- M2 spacer holes may trigger `npth_inside_courtyard`, `courtyards_overlap`, duplicate `Ref**`, or extra footprint warnings near switches such as SW5, SW17, and SW20. The important check is whether the spacer hole actually overlaps switch lands, plated through holes, or functional switch holes. Courtyard-only overlap is non-blocking.
- Silkscreen clipping and silk-over-copper findings are cosmetic unless they reveal a real placement mistake.
- Prioritize actual electrical/connectivity failures, shorts, real copper/hole clearance problems, and unconnected matrix nets. In the 2026-06-28 left-side DRC check, the real remaining blockers were unconnected `Col3` and `Col4`.

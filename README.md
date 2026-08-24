# ReeL

Hardware sources and manufacturing assets for the ReeL split keyboard.

The RMK firmware, Vial definition, trackball profiler, and firmware build
workflow live in [rmk-config-ReeL](https://github.com/cotrin8672/rmk-config-ReeL).

## Repository layout

- `pcb/`: KiCad projects, project-local symbol and footprint libraries, and the PMW3610 breakout manufacturing package
- `model/`: mechanical STEP and STL models
- `fabrication/gerber/`: generated Gerber and drill outputs for the main left and right PCBs
- `docs/materials/`: component research and supporting material documents

The firmware repository was split from hardware commit
`b9bacc7681b5a1461f23b08cf7a2b1b867c73b0e`.

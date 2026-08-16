#!/usr/bin/env python3
"""Regenerate the Vial layout from the ReeL PCB matrix netlists.

The KiCad boards are the source of truth for the logical matrix positions:
switches are joined to a ``ColN`` net and to a diode net whose diode is joined
to ``RowN``.  The right half is offset by six columns in the unified matrix.

Vial's ``layouts.keymap`` is a display order, not a PCB/netlist format.  The
display geometry is therefore derived from the switch positions and the
``Edge.Cuts`` bounds.  Every key is emitted at its PCB-derived position in
18 mm key units, including the stagger and the angled thumb switches.  This
keeps the unusual split shape tied to the PCB instead of a second hand-written
layout source.
"""

from __future__ import annotations

import argparse
import json
import math
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "vial.json"
PCB_SOURCES = (
    (ROOT / "hardware" / "pcb" / "left" / "reel-left.kicad_pcb", 0, "left"),
    (ROOT / "hardware" / "pcb" / "right" / "reel-right.kicad_pcb", 6, "right"),
)

KEY_PITCH_MM = 18.0
# Vial has no PCB-outline primitive. Place the two Edge.Cuts bounding boxes
# edge-to-edge so the key spacing reflects the board envelope rather than a
# hand-tuned one-key split gap.
BOARD_GAP_MM = 0.0

FOOTPRINT_START = re.compile(r'^\s*\(footprint\s+"([^"]+)"')
REFERENCE = re.compile(r'^\s*\(property\s+"Reference"\s+"([^"]+)"')
AT = re.compile(
    r'^\s*\(at\s+([-+]?\d+(?:\.\d+)?)\s+'
    r'([-+]?\d+(?:\.\d+)?)(?:\s+([-+]?\d+(?:\.\d+)?))?'
)
GRAPHIC_START = re.compile(r'^\s*\(gr_(?:line|arc|rect|poly)\b')
EDGE_POINT = re.compile(
    r'\((?:start|mid|end)\s+([-+]?\d+(?:\.\d+)?)\s+'
    r'([-+]?\d+(?:\.\d+)?)'
)
NET = re.compile(r'\(net\s+(?:(?:\d+)\s+)?"([^"]+)"\)')
MATRIX_SWITCH = re.compile(r'^SW\d+$')
MATRIX_DIODE = re.compile(r'^D\d+$')
ROW_NET = re.compile(r'^Row(\d+)$')
COL_NET = re.compile(r'^Col(\d+)$')


@dataclass(frozen=True)
class BoardBounds:
    min_x: float
    max_x: float
    min_y: float
    max_y: float

    @property
    def width(self) -> float:
        return self.max_x - self.min_x


@dataclass(frozen=True)
class Footprint:
    reference: str
    x: float
    y: float
    angle: float
    nets: frozenset[str]


@dataclass(frozen=True)
class MatrixKey:
    row: int
    col: int
    reference: str
    x: float
    y: float
    layout_x: float
    layout_y: float
    angle: float

    @property
    def coordinate(self) -> str:
        return f"{self.row},{self.col}"


def paren_delta(line: str) -> int:
    """Count parentheses outside quoted strings on one KiCad line."""

    delta = 0
    quoted = False
    escaped = False
    for character in line:
        if escaped:
            escaped = False
        elif character == "\\" and quoted:
            escaped = True
        elif character == '"':
            quoted = not quoted
        elif not quoted and character == "(":
            delta += 1
        elif not quoted and character == ")":
            delta -= 1
    return delta


def read_footprints(path: Path) -> list[Footprint]:
    """Read only footprint references, positions, and pad net names."""

    footprints: list[Footprint] = []
    current_name: str | None = None
    current_lines: list[str] = []
    current_depth = 0

    def finish() -> None:
        if current_name is None:
            return
        reference = next(
            (match.group(1) for line in current_lines if (match := REFERENCE.match(line))),
            None,
        )
        position = next(
            (match for line in current_lines if (match := AT.match(line))),
            None,
        )
        if reference is None or position is None:
            return
        nets = frozenset(match.group(1) for line in current_lines for match in NET.finditer(line))
        footprints.append(
            Footprint(
                reference=reference,
                x=float(position.group(1)),
                y=float(position.group(2)),
                angle=float(position.group(3) or 0),
                nets=nets,
            )
        )

    for line in path.read_text(encoding="utf-8").splitlines():
        start = FOOTPRINT_START.match(line)
        if start:
            if current_name is not None:
                raise ValueError(f"unclosed footprint before {start.group(1)} in {path}")
            finish()
            current_name = start.group(1)
            current_lines = [line]
            current_depth = paren_delta(line)
            continue
        if current_name is not None:
            current_lines.append(line)
            current_depth += paren_delta(line)
            if current_depth == 0:
                finish()
                current_name = None
                current_lines = []

    if current_name is not None:
        finish()
    if not footprints:
        raise ValueError(f"no footprints found in {path}")
    return footprints


def read_edge_bounds(path: Path) -> BoardBounds:
    """Read the bounding box of the board's Edge.Cuts graphics."""

    bounds = [math.inf, -math.inf, math.inf, -math.inf]
    current_lines: list[str] = []
    current_depth = 0
    in_graphic = False

    def finish() -> None:
        if not in_graphic:
            return
        block = "\n".join(current_lines)
        if '(layer "Edge.Cuts")' not in block:
            return
        for match in EDGE_POINT.finditer(block):
            x = float(match.group(1))
            y = float(match.group(2))
            bounds[0] = min(bounds[0], x)
            bounds[1] = max(bounds[1], x)
            bounds[2] = min(bounds[2], y)
            bounds[3] = max(bounds[3], y)

    for line in path.read_text(encoding="utf-8").splitlines():
        if GRAPHIC_START.match(line):
            if in_graphic:
                raise ValueError(f"unclosed Edge.Cuts graphic in {path}")
            current_lines = [line]
            current_depth = paren_delta(line)
            in_graphic = True
            continue
        if not in_graphic:
            continue
        current_lines.append(line)
        current_depth += paren_delta(line)
        if current_depth == 0:
            finish()
            current_lines = []
            in_graphic = False

    if in_graphic:
        finish()
    if not all(math.isfinite(value) for value in bounds):
        raise ValueError(f"no Edge.Cuts graphics found in {path}")
    return BoardBounds(*bounds)


def derive_matrix_keys(
    path: Path,
    column_offset: int,
    board_x_offset: float,
) -> list[MatrixKey]:
    footprints = read_footprints(path)
    bounds = read_edge_bounds(path)
    diode_rows: dict[str, int] = {}

    for footprint in footprints:
        if not MATRIX_DIODE.fullmatch(footprint.reference):
            continue
        row_nets = [net for net in footprint.nets if ROW_NET.fullmatch(net)]
        diode_nets = [net for net in footprint.nets if net.startswith("Net-(D")]
        if len(row_nets) != 1 or len(diode_nets) != 1:
            continue
        diode_rows[diode_nets[0]] = int(ROW_NET.fullmatch(row_nets[0]).group(1))

    switch_records: list[tuple[Footprint, int, int]] = []
    for footprint in footprints:
        if not MATRIX_SWITCH.fullmatch(footprint.reference):
            continue
        col_nets = [net for net in footprint.nets if COL_NET.fullmatch(net)]
        diode_nets = [net for net in footprint.nets if net in diode_rows]
        if not col_nets and not diode_nets:
            # Rotary encoders, power switches, and other auxiliary footprints
            # are not matrix switches.
            continue
        if len(col_nets) != 1 or len(diode_nets) != 1:
            raise ValueError(
                f"{path.name}: {footprint.reference} does not have exactly one "
                "column net and one diode net"
            )
        row = diode_rows[diode_nets[0]]
        local_col = int(COL_NET.fullmatch(col_nets[0]).group(1))
        switch_records.append((footprint, row, local_col))

    if not switch_records:
        raise ValueError(f"no matrix switches found in {path}")

    keys: list[MatrixKey] = []
    for footprint, row, local_col in switch_records:
        col = local_col + column_offset
        keys.append(
            MatrixKey(
                row=row,
                col=col,
                reference=footprint.reference,
                x=footprint.x,
                y=footprint.y,
                layout_x=board_x_offset
                + (footprint.x - bounds.min_x) / KEY_PITCH_MM,
                layout_y=(footprint.y - bounds.min_y) / KEY_PITCH_MM,
                angle=footprint.angle,
            )
        )
    return keys


def derive_all_matrix_keys() -> dict[str, MatrixKey]:
    keys: dict[str, MatrixKey] = {}
    positions: dict[tuple[float, float], str] = {}
    left_bounds = read_edge_bounds(PCB_SOURCES[0][0])
    board_x_offsets = {"left": 0.0, "right": (left_bounds.width + BOARD_GAP_MM) / KEY_PITCH_MM}
    for path, column_offset, side in PCB_SOURCES:
        for key in derive_matrix_keys(path, column_offset, board_x_offsets[side]):
            if key.coordinate in keys:
                raise ValueError(f"duplicate matrix coordinate {key.coordinate}")
            position = (round(key.x, 4), round(key.y, 4))
            if position in positions:
                raise ValueError(
                    f"duplicate switch position {position}: "
                    f"{positions[position]} and {key.reference}"
                )
            keys[key.coordinate] = key
            positions[position] = key.reference
    return keys


def build_display_layout(keys: dict[str, MatrixKey]) -> tuple[tuple[object, ...], ...]:
    """Build absolute KLE-style positions from the PCB switch geometry."""

    ordered = sorted(
        keys.values(),
        key=lambda key: (key.layout_y, key.layout_x, key.row, key.col),
    )
    display_rows: list[tuple[object, ...]] = []
    for key in ordered:
        angle = key.angle % 360
        if math.isclose(angle % 180, 0, abs_tol=1e-6):
            angle = 0
        elif angle > 180:
            angle -= 360
        display_rows.append(
            (
                {
                    "r": round(angle, 4),
                    "rx": round(key.layout_x, 4),
                    "ry": round(key.layout_y, 4),
                    "x": -0.5,
                    "y": -0.5,
                },
                key.coordinate,
            )
        )
    return tuple(display_rows)


def build_keymap(keys: dict[str, MatrixKey]) -> list[list[object]]:
    display_layout = build_display_layout(keys)
    layout_keys = [
        item
        for row in display_layout
        for item in row
        if isinstance(item, str)
    ]
    pcb_keys = set(keys)
    missing = sorted(pcb_keys - set(layout_keys))
    unexpected = sorted(set(layout_keys) - pcb_keys)
    duplicate = len(layout_keys) != len(set(layout_keys))
    if missing or unexpected or duplicate:
        details = []
        if missing:
            details.append(f"missing from display layout: {', '.join(missing)}")
        if unexpected:
            details.append(f"not present in PCB: {', '.join(unexpected)}")
        if duplicate:
            details.append("duplicate coordinate in display layout")
        raise ValueError("generated display layout does not match PCB (" + "; ".join(details) + ")")

    keymap: list[list[object]] = []
    for row in display_layout:
        output_row: list[object] = []
        for item in row:
            if isinstance(item, str):
                output_row.append(item)
            elif isinstance(item, dict) and {"r", "rx", "ry", "x", "y"}.issubset(item):
                output_row.append(item)
            else:
                raise ValueError(f"unsupported Vial layout item: {item!r}")
        keymap.append(output_row)
    return keymap


def generate() -> dict[str, object]:
    keys = derive_all_matrix_keys()
    keymap = build_keymap(keys)
    config = json.loads(OUTPUT.read_text(encoding="utf-8"))
    config["name"] = "ReeL"
    config["matrix"] = {
        "rows": max(key.row for key in keys.values()) + 1,
        "cols": max(key.col for key in keys.values()) + 1,
    }
    config["layouts"] = {"keymap": keymap}
    return config


def encoded(config: dict[str, object]) -> str:
    return json.dumps(config, ensure_ascii=False, indent=2) + "\n"


def main(argv: Iterable[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="fail instead of writing when vial.json is not up to date",
    )
    args = parser.parse_args(argv)

    try:
        rendered = encoded(generate())
        current = OUTPUT.read_text(encoding="utf-8")
        if args.check:
            if current != rendered:
                print(f"{OUTPUT} is out of date", file=sys.stderr)
                return 1
            print(f"{OUTPUT} is up to date")
            return 0
        with OUTPUT.open("w", encoding="utf-8", newline="\n") as stream:
            stream.write(rendered)
        print(f"generated {OUTPUT}")
        return 0
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())

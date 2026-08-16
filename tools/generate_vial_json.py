#!/usr/bin/env python3
"""Regenerate the Vial layout from the ReeL PCB matrix netlists.

The KiCad boards are the source of truth for the logical matrix positions:
switches are joined to a ``ColN`` net and to a diode net whose diode is joined
to ``RowN``.  The right half is offset by six columns in the unified matrix.

Vial's ``layouts.keymap`` is a display order, not a PCB/netlist format.  The
display order is therefore derived from the switch positions: the top rows
are ordered by PCB X position, and the mirrored thumb keys use their physical
X/Y positions to select the bottom or the row-1/row-2 extension.  This keeps
the unusual row-3 placement tied to the PCB instead of a second hand-written
layout source.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from statistics import median
from typing import Iterable


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "vial.json"
PCB_SOURCES = (
    (ROOT / "hardware" / "pcb" / "left" / "reel-left.kicad_pcb", 0),
    (ROOT / "hardware" / "pcb" / "right" / "reel-right.kicad_pcb", 6),
)

LEFT_MATRIX_COLUMNS = 6
TOP_ROWS = (0, 1, 2)

FOOTPRINT_START = re.compile(r'^\s*\(footprint\s+"([^"]+)"')
REFERENCE = re.compile(r'^\s*\(property\s+"Reference"\s+"([^"]+)"')
AT = re.compile(
    r'^\s*\(at\s+([-+]?\d+(?:\.\d+)?)\s+'
    r'([-+]?\d+(?:\.\d+)?)(?:\s+([-+]?\d+(?:\.\d+)?))?'
)
NET = re.compile(r'\(net\s+(?:(?:\d+)\s+)?"([^"]+)"\)')
MATRIX_SWITCH = re.compile(r'^SW\d+$')
MATRIX_DIODE = re.compile(r'^D\d+$')
ROW_NET = re.compile(r'^Row(\d+)$')
COL_NET = re.compile(r'^Col(\d+)$')


@dataclass(frozen=True)
class Footprint:
    reference: str
    x: float
    y: float
    nets: frozenset[str]


@dataclass(frozen=True)
class MatrixKey:
    row: int
    col: int
    reference: str
    x: float
    y: float

    @property
    def coordinate(self) -> str:
        return f"{self.row},{self.col}"


def read_footprints(path: Path) -> list[Footprint]:
    """Read only footprint references, positions, and pad net names."""

    footprints: list[Footprint] = []
    current_name: str | None = None
    current_lines: list[str] = []
    current_depth = 0

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


def derive_matrix_keys(path: Path, column_offset: int) -> list[MatrixKey]:
    footprints = read_footprints(path)
    diode_rows: dict[str, int] = {}

    for footprint in footprints:
        if not MATRIX_DIODE.fullmatch(footprint.reference):
            continue
        row_nets = [net for net in footprint.nets if ROW_NET.fullmatch(net)]
        diode_nets = [net for net in footprint.nets if net.startswith("Net-(D")]
        if len(row_nets) != 1 or len(diode_nets) != 1:
            continue
        diode_rows[diode_nets[0]] = int(ROW_NET.fullmatch(row_nets[0]).group(1))

    keys: list[MatrixKey] = []
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
        col = int(COL_NET.fullmatch(col_nets[0]).group(1)) + column_offset
        keys.append(MatrixKey(row, col, footprint.reference, footprint.x, footprint.y))

    if not keys:
        raise ValueError(f"no matrix switches found in {path}")
    return keys


def derive_all_matrix_keys() -> dict[str, MatrixKey]:
    keys: dict[str, MatrixKey] = {}
    positions: dict[tuple[float, float], str] = {}
    for path, column_offset in PCB_SOURCES:
        for key in derive_matrix_keys(path, column_offset):
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


def by_x(keys: Iterable[MatrixKey], *, reverse: bool = False) -> list[MatrixKey]:
    return sorted(
        keys,
        key=lambda key: (key.x, key.y, key.coordinate),
        reverse=reverse,
    )


def build_display_layout(keys: dict[str, MatrixKey]) -> tuple[tuple[object, ...], ...]:
    """Build Vial's display rows from the PCB positions and matrix rows."""

    left = [key for key in keys.values() if key.col < LEFT_MATRIX_COLUMNS]
    right = [key for key in keys.values() if key.col >= LEFT_MATRIX_COLUMNS]
    left_by_row = {
        row: by_x(key for key in left if key.row == row) for row in TOP_ROWS
    }
    right_by_row = {
        row: by_x(key for key in right if key.row == row) for row in TOP_ROWS
    }
    if any(not left_by_row[row] or not right_by_row[row] for row in TOP_ROWS):
        raise ValueError("PCB is missing a top-row matrix switch on one half")

    right_row3 = [key for key in right if key.row == 3]
    right_regular_x = [key.x for row in TOP_ROWS for key in right_by_row[row]]
    right_outer = [key for key in right_row3 if key.x > max(right_regular_x)]
    right_bottom = by_x(
        (key for key in right_row3 if key not in right_outer),
        reverse=True,
    )
    row_centers = {
        row: median(key.y for key in right_by_row[row]) for row in (1, 2)
    }
    outer_by_row: dict[int, list[MatrixKey]] = {1: [], 2: []}
    for key in by_x(right_outer):
        target_row = min(row_centers, key=lambda row: abs(key.y - row_centers[row]))
        outer_by_row[target_row].append(key)

    if len(right_outer) != 2 or any(len(outer_by_row[row]) != 1 for row in (1, 2)):
        raise ValueError(
            "expected one right-half row-3 outer key beside each of rows 1 and 2"
        )

    display_rows: list[tuple[object, ...]] = []
    for row in TOP_ROWS:
        display_rows.append(
            tuple(
                [key.coordinate for key in left_by_row[row]]
                + [{"x": 1}]
                + [key.coordinate for key in right_by_row[row]]
                + [key.coordinate for key in outer_by_row.get(row, [])]
            )
        )
    display_rows.append(
        tuple(
            [key.coordinate for key in by_x((key for key in left if key.row == 3), reverse=True)]
            + [{"x": 2}]
            + [key.coordinate for key in right_bottom]
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
            elif isinstance(item, dict) and set(item) == {"x"}:
                output_row.append({"x": item["x"]})
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

#!/usr/bin/env python3
"""Regenerate the two original, monochrome 3x status-item PNG fixtures."""

import math
from pathlib import Path
import struct
import zlib


def capsule(x, y, left, top, width, height):
    radius = width / 2
    nearest_y = min(max(y, top + radius), top + height - radius)
    return (x - left - radius) ** 2 + (y - nearest_y) ** 2 <= radius**2


def write_icon(path, running):
    side = 54
    rows = bytearray()
    for y in range(side):
        rows.append(0)
        for x in range(side):
            covered = 0
            for sub_y in range(4):
                for sub_x in range(4):
                    point_x = (x + (sub_x + 0.5) / 4) / 3
                    point_y = (y + (sub_y + 0.5) / 4) / 3
                    waveform = any(
                        capsule(point_x, point_y, *bar)
                        for bar in ((2, 6, 2, 6), (6, 3, 2, 12), (10, 5, 2, 8))
                    )
                    marker = running and math.hypot(point_x - 15, point_y - 4) <= 2
                    covered += waveform or marker
            rows.extend((0, 0, 0, round(255 * covered / 16)))

    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))

    path.write_bytes(
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", side, side, 8, 6, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(bytes(rows)))
        + chunk(b"IEND", b"")
    )


for name, running in (("idle", False), ("running", True)):
    write_icon(Path(__file__).with_name(f"{name}.png"), running)

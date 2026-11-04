#!/usr/bin/env python3
"""Regenerate docs/screenshots from a release build. Run from the repository root.

Times are chosen to land in daylight, and a fixed seed keeps the frames reproducible.
"""
import os
import subprocess
import tempfile

from PIL import Image

OUT = "docs/screenshots"
SEED = "7"
# name, world, seconds, cols, rows, pixel, cell width, cell height
SHOTS = [
    ("aquarium-pixel", "aquarium", 260, 440, 130, True, 5, 10),
    ("aquarium-ascii", "aquarium", 260, 170, 48, False, 9, 18),
    ("zoo-pixel", "zoo", 60, 440, 130, True, 5, 10),
    ("village-pixel", "village", 60, 440, 130, True, 5, 10),
    ("traffic-pixel", "traffic", 320, 440, 130, True, 5, 10),
    ("city-pixel", "city", 60, 440, 130, True, 5, 10),
]
# Close-up of the sea bed, cropped from the aquarium frame: left, top, right, bottom.
SEABED = (0, 1000, 1200, 1300)


def main():
    with tempfile.TemporaryDirectory() as tmp:
        for name, world, secs, cols, rows, pixel, cw, ch in SHOTS:
            cells = os.path.join(tmp, name + ".txt")
            env = {**os.environ, "SEED": SEED, "OUT": cells, "WORLD": f"{cols}x{rows}"}
            env.pop("PIXEL", None)
            if pixel:
                env["PIXEL"] = "1"
            subprocess.run(
                ["target/release/ascii-zoo", "--dump", world, str(secs), str(cols), str(rows)],
                env=env,
                stdout=subprocess.DEVNULL,
                check=True,
            )
            png = f"{OUT}/{name}.png"
            subprocess.run(["python3", "tools/render.py", cells, png, str(cw), str(ch)], check=True)
    Image.open(f"{OUT}/aquarium-pixel.png").crop(SEABED).save(f"{OUT}/aquarium-seabed.png")


if __name__ == "__main__":
    main()

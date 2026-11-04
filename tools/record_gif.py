#!/usr/bin/env python3
"""Record the README's animated 3D clip from a release build. Run from the repository root.

Each frame is a separate `--dump` run. A fixed seed makes every run replay the same
world, so the frames join into one continuous clip. Needs Pillow and ffmpeg.
"""
import math
import os
import subprocess
import tempfile
from concurrent.futures import ThreadPoolExecutor

OUT = "docs/screenshots/aquarium-3d.gif"
SEED = "7"
WORLD, COLS, ROWS = "240x80", 240, 68
# Starts in daylight, like the aquarium stills.
START = 260.0
FPS = 15
SECONDS = 10
CELL_W, CELL_H = 3, 6
YAW, YAW_SWING, PITCH = 0.0, 0.5, 0.45


def frame(tmp, i):
    # The camera sways on a sine so the clip's motion loops without a jump.
    yaw = YAW + YAW_SWING * math.sin(2 * math.pi * i / (FPS * SECONDS))
    cells = os.path.join(tmp, f"{i:04}.txt")
    env = {**os.environ, "SEED": SEED, "OUT": cells, "WORLD": WORLD, "PIXEL": "1", "CAM3D": f"{yaw},{PITCH}"}
    secs = START + i / FPS
    subprocess.run(
        ["target/release/ascii-zoo", "--dump", "aquarium", str(secs), str(COLS), str(ROWS)],
        env=env,
        stdout=subprocess.DEVNULL,
        check=True,
    )
    subprocess.run(
        ["python3", "tools/render.py", cells, os.path.join(tmp, f"{i:04}.png"), str(CELL_W), str(CELL_H)],
        check=True,
    )
    os.remove(cells)


def main():
    with tempfile.TemporaryDirectory() as tmp:
        with ThreadPoolExecutor(max_workers=os.cpu_count()) as pool:
            list(pool.map(lambda i: frame(tmp, i), range(FPS * SECONDS)))
        palette = "split[a][b];[a]palettegen=stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=4"
        subprocess.run(
            ["ffmpeg", "-y", "-loglevel", "error", "-framerate", str(FPS), "-i", os.path.join(tmp, "%04d.png"), "-vf", palette, "-loop", "0", OUT],
            check=True,
        )


if __name__ == "__main__":
    main()

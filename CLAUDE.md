# ascii-zoo

Terminal simulation engine in Rust and ratatui. See README.md for what it does and its keys.

## Git

- Commit as `mjgalindo <mjgalindoramos@gmail.com>`, never a @kape.com address.
- Commits are unsigned.
- Remote is `git@github.com:mjgalindo/ascii-life.git`. Commit or push only when asked.

## Commands

```
cargo build --release
cargo test --release
cargo clippy
target/release/ascii-zoo --bench <world> <seconds> <w> <h>
target/release/ascii-zoo --dump <world> <seconds> <cols> <rows>
python3 tools/screenshots.py
```

`--dump` takes `WORLD=WxH`, `VIEW=x,y,zoom`, `PIXEL=1`, `CAM3D=yaw,pitch`, `RIDE=kind`, `SEED=n` and `OUT=file`. Worlds are aquarium, traffic, zoo, village, city and gallery.

## Layout

- `src/main.rs`: arguments, TUI loop, input, dump and bench modes.
- `src/ui.rs`: ticker, log, help, inspector, minimap, tools panel.
- `src/engine/`: entities, sprites, world update, spatial snapshot, rendering. Knows nothing about specific worlds.
- `src/scenarios/`: one file per world, registered in `mod.rs`. `traffic.rs` also provides the city.
- `src/engine/render3d.rs`: perspective view, render only. `Scenario::space` picks how a world maps to 3D.
- `src/art.rs`: procedural pixel art.

## Rules that are easy to break

- **Determinism**: behaviors update in parallel chunks, and the same seed must give the same world on any thread count. Use `cx.rng`, and change other entities only through `Ctx`.
- **Scan order is not neutral**: `Snapshot::each` and `query` visit from the top left. Anything that stops early or takes the first match steers creatures that way, and rectangle overlap reaches further toward wide sprites. Measure range between centers and pick by distance.
- **`pos` is the top-left corner**: use `e.cx()` and `e.cy()` for distances and steering.
- **Per-tick cost matters**: worlds reach tens of thousands of entities, so avoid allocating in `update`. Prefer `each` to `query` on hot paths, and `of_kind` and `count` to scanning. Check changes with `--bench`.
- **Both render modes**: a new sprite needs to read well as ASCII and as pixels. Overlay text is drawn as glyphs only in pixel mode.

## Verifying behavior

Tests are few, so check simulation changes by measuring. Run `--dump` across several seeds and count what you care about, and compare before and after. One frame says little, because schools and herds move as groups. Population changes need runs of 30 simulated minutes at small and large sizes.

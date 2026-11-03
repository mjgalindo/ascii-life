# ascii-zoo

A tiny ASCII simulation engine in Rust (ratatui), in the spirit of asciiquarium. Worlds are *scenarios*: each one only defines its entities and spawn rules.

```
cargo run --release [aquarium|traffic|zoo|village|city]
cargo run --release -- city --world 600x200 --pixel   # big world, half-block pixel art
cargo run --release -- --dump zoo 30 120 40          # headless: print one frame after 30s
cargo run --release -- --bench city 60 600 200       # simulation throughput
`WORLD=WxH VIEW=x,y,zoom PIXEL=1 OUT=cells.txt` tune `--dump`; `tools/render.py cells.txt out.png` turns a dump into an image.
```

The `city` world combines them all: every city block is a diorama (zoo pen, fish tank, farmstead, park) between signalised roads with cars, pedestrians and emergency vehicles.

Keys: `tab`/`1-6` switch world, `m` toggles ASCII/pixel rendering, arrows or drag pan, `z`/`x` or wheel zoom, `g` fit whole world, click inspects (`f` follows), `+`/`-` change speed (unbounded), `t` turbo (use all CPU), `r` reset, `space` pause, `q` quit.

## Engine (`src/engine`)

- **Sprite**: multi-line art, optional color mask, animation frames, auto-mirroring for left/right facing.
- **Entity**: position, velocity, z-depth, animation, hitbox, lifetime, `die_offscreen`, `follow` (compound objects).
- **Behavior**: `update`, `on_collide`, `on_death` hooks. Neighbors are seen through a per-tick spatial snapshot (`cx.snap.query`).
- **Scenario**: `setup` populates the world, `tick` spawns life over time, optional day/night cycle.

## Adding a world

1. Create `src/scenarios/mything.rs` with a type implementing `Scenario`.
2. Register it in `src/scenarios/mod.rs`.

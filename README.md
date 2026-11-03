# ascii-zoo

A tiny ASCII simulation engine in Rust (ratatui), in the spirit of asciiquarium. Worlds are *scenarios*: each one only defines its entities and spawn rules.

```
cargo run --release [aquarium|traffic|zoo|village|city]
cargo run --release -- --dump zoo 30 120 40   # headless: print one frame after 30s
```

The `city` world combines them all: every city block is a diorama (zoo pen, fish tank, farmstead, park) between signalised roads with cars, pedestrians and emergency vehicles.

Keys: `tab`/`1-5` switch world, click inspects an entity, `r` reset, `space` pause, `+`/`-` speed, `q` quit.

## Engine (`src/engine`)

- **Sprite**: multi-line art, optional color mask, animation frames, auto-mirroring for left/right facing.
- **Entity**: position, velocity, z-depth, animation, hitbox, lifetime, `die_offscreen`, `follow` (compound objects).
- **Behavior**: `update`, `on_collide`, `on_death` hooks. Neighbors are seen through a per-tick spatial snapshot (`cx.snap.query`).
- **Scenario**: `setup` populates the world, `tick` spawns life over time, optional day/night cycle.

## Adding a world

1. Create `src/scenarios/mything.rs` with a type implementing `Scenario`.
2. Register it in `src/scenarios/mod.rs`.

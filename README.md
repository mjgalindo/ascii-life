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

Keys: `tab`/`1-6` switch world, `m` toggles ASCII/pixel rendering, arrows or drag pan, `z`/`x` or wheel zoom, `g` fit whole world, click inspects (`f` follows), `c` director camera, `j` jump to latest news, `l` news log, `n` minimap, `a` add things (pick a number, click to place more), `+`/`-` change speed (unbounded), `t` turbo (use all CPU), `r` reset, `space` pause, `?` help, `q` quit.

## What goes on in there

- **aquarium**: a food chain. Sunlight grows plankton, schools of fish flock, graze, breed and starve, and sharks turn up when fish are plentiful. Ships sink and leave treasure wrecks that divers raid. Crabs, jellyfish and anglerfish live around the kelp.
- **zoo**: herds get hungry and eat from troughs, and keepers go wherever food runs low. Babies are born and follow their mothers. Animals escape through gaps or lightning strikes, visitors panic, predators stalk them and rangers bring them home. Daily parades and reputation decide how many people come.
- **village**: villagers are born, work, grow up into a trade and die. Woodcutters fell trees that regrow, lumber builds houses, and caravans trade lumber for grain. Winter brings famine and wolves, guards chase the wolves off, and everybody runs to fight house fires. Footpaths wear into the grass where people walk, and autumn has a festival.
- **traffic**: two rush hours a day, buses with stops, rain that slows cars and causes crashes, tow trucks that clear the wrecks, and honking.

## Engine (`src/engine`)

- **Sprite**: multi-line art, optional color mask, animation frames, auto-mirroring for left/right facing.
- **Entity**: position, velocity, z-depth, animation, hitbox, lifetime, `die_offscreen`, `follow` (compound objects).
- **Behavior**: `update`, `on_collide` (with `wants_collisions`), `on_death` and `describe` hooks. Neighbors are seen through a per-tick spatial snapshot (`cx.snap.query`).
- **Scenario**: `setup` populates the world, `tick` spawns life over time, optional day/night cycle. `cx.news(text, pos)` publishes a headline to the ticker.

## Adding a world

Implement `tools` and `place` on your scenario to fill its add menu.

1. Create `src/scenarios/mything.rs` with a type implementing `Scenario`.
2. Register it in `src/scenarios/mod.rs`.

# Diorama map core (MIT)

The files in this directory are a Rust port of the angled minimap of
[Diorama](https://github.com/SunkenInTime/diorama) by Dara Adedeji (MIT). They are the
`diorama-core` crate of the unofficial Horizon port by Worrcc,
https://github.com/Worrcc/diorama-horizon (commit `7a3825b`, also MIT), and keep its file layout,
formulas and authorship. `LICENSE` is that port's license text with both copyright lines, unchanged.

- Taken: `projection`, `style`, `looks`, `bake`, `cave`, `layout`, `paint` and `mosaic`: the oblique
  camera (50 degrees, facing north), the column painter (top and south faces), the style's light, grade
  and outlines, the block-look table, the cave and Nether slices and the mosaic of baked chunks.
- Not taken: the port's WASM glue (`wasm/`, written for the Horizon-iOS fork's mod API, which official
  Horizon Nightlies do not have), its settings, JSON, waypoint and budget modules, and the benchmark.
- Local changes to the upstream files: the `crate::` paths are rewritten to the sibling modules of this
  directory (`super::diorama_*`), and `DEFAULT_PITCH_DEGREES` moved into `projection.rs`, two nested `if`s in `looks.rs` are collapsed
  for Clippy, `cargo fmt` formatting is applied, and the file-level `//!` doc comments are plain `//` comments (the mixin tool rejects file attributes).
- Known look differences from Diorama itself (see the port's RULES.md and PLAN-v2.md): columns are painted
  instead of every pixel being ray-traced, so an overhang is drawn as a column down to the ground; there are
  no sun shadow rays and no corner occlusion rays.

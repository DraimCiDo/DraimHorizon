**DraimHorizon 0.40.8 - 3D minimap (Diorama), step 2**

Target: official Horizon Nightly source `9f42064` (unchanged).

- New switch **Map view: 3D / flat** in J → Claims → Map settings (off by default). In 3D the minimap shows the world at an angle, like a little model: block tops and south faces in the game's own textures with biome tints, sun light, height shade and outlines (the Diorama look). Surface view only; zoom up to 3 blocks per pixel.
- The M map and remembered terrain are still flat; they are the next steps.

Verification: Clippy with warnings denied, cargo fmt and the workspace tests pass. The pinned native client was staged and `cargo check`ed against Nightly 9f42064 with the new module. A running game was not checked, so the 3D look, its speed and the mark positions are untested in play.

**DraimHorizon 0.40.11 - 3D world map, sharper minimap**

Target: official Horizon Nightly source `9f42064` (unchanged).

- The M map has a **Map view: 3D / flat** button. In 3D, the terrain around you (up to about 380 blocks across) is drawn at an angle like the minimap; farther saved terrain stays flat. Marks follow the angled view.
- The 3D minimap is sharpened: edges and outlines are crisper.

Verification: Clippy with warnings denied, cargo fmt and the workspace tests pass. Not checked in a running game.

**DraimHorizon 0.40.10 - 3D minimap: sharper picture**

Target: official Horizon Nightly source `9f42064` (unchanged).

- The 3D minimap is drawn from finer detail (four texels per block) and averaged down per pixel, so edges and textures are smooth instead of blocky. Widest zoom in 3D is now 2.5 blocks per pixel. The picture itself is still the host's fixed 128 x 128 minimap sprite.

Verification: Clippy with warnings denied, cargo fmt and the workspace tests pass. Not checked in a running game.

**DraimHorizon 0.40.9 - 3D minimap: shadows and occlusion**

Target: official Horizon Nightly source `9f42064` (unchanged).

- The 3D minimap now casts sun shadows (hills, trees and buildings shade what is behind them) and darkens block corners and pits, as Diorama does. Shadows are per block column, so they are coarser than Diorama's own.

Verification: Clippy with warnings denied, cargo fmt and the workspace tests pass. Not checked in a running game.

**DraimHorizon 0.40.8 - 3D minimap (Diorama), step 2**

Target: official Horizon Nightly source `9f42064` (unchanged).

- New switch **Map view: 3D / flat** in J → Claims → Map settings (off by default). In 3D the minimap shows the world at an angle, like a little model: block tops and south faces in the game's own textures with biome tints, sun light, height shade and outlines (the Diorama look). Surface view only; zoom up to 3 blocks per pixel.
- The M map and remembered terrain are still flat; they are the next steps.

Verification: Clippy with warnings denied, cargo fmt and the workspace tests pass. The pinned native client was staged and `cargo check`ed against Nightly 9f42064 with the new module. A running game was not checked, so the 3D look, its speed and the mark positions are untested in play.

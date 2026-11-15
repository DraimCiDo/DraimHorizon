**DraimHorizon 0.40.8 - 3D map (Diorama)**

Target: official Horizon Nightly source `9f42064` (unchanged).

- New switch **Map view: 3D / flat** in J → Claims → Map settings and as a button in the M map's toolbar (off by default).
- In 3D the minimap shows the world at an angle, like a little model: block tops and south faces in the game's own textures with biome tints, sun light and shadows, corner shading, height shade and outlines (the Diorama look). Surface view only; zoom up to 2.5 blocks per pixel. The picture is the host's fixed 128 x 128 sprite, drawn from finer detail, averaged down and sharpened.
- The M map draws the terrain around you (up to about 380 blocks across) at an angle too; farther saved terrain stays flat. Marks follow the angled view.

Verification: Clippy with warnings denied, cargo fmt and the workspace tests pass. The pinned native client was staged and `cargo check`ed against Nightly 9f42064. A running game was not checked, so the 3D look, its speed and the mark positions are untested in play.

**DraimHorizon 0.40.7 - Diorama map core**

Target: official Horizon Nightly source `9f42064` (unchanged).

- Adds the pure core of the Diorama angled map (projection, style, block looks, column painter, cave slice, chunk mosaic) under `mixins/draimhorizon/diorama/`, MIT, with the original authors' license and a NOTICE. It is not used by the minimap or the M map yet, so nothing changes in game; the next steps wire it in.

Verification: Clippy with warnings denied, cargo fmt and the workspace tests pass (the core's 66 tests included). The pinned native client was staged and `cargo check`ed against Nightly 9f42064 with the new modules. A running game was not checked.

# DraimHorizon 0.44.4

**Fast head turns for every automation click.** Opening a chest, placing a block, breaking a shulker box or aiming at an ore now turns the head straight to the target at 60° a tick (a half turn in about 0.15 s) and clicks in the same tick it arrives, whatever the Human look setting.

- A hand resting on the mouse no longer pauses the aim: only a real mouse move (12 pixels within 0.4 s) takes the camera. Before, a single stray pixel stopped every task from aiming for three seconds.
- With Human look on, the head's up/down movement while walking is three times faster.
- The sorter stops a stuck walk before trying again and never aims while a walk is still moving the player; its status says when it waits for you to let go of the camera.

Earlier fixes in 0.44.2 and 0.44.3: double chests open from either side, clicks never go through item frames, no false "full", non-stop **Watch**, and messages naming any container the sorter skips.

The Horizon package targets the official Nightly `8f83e5c`; the Sunrise package targets Nightly `9425651`.

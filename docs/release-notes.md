# DraimHorizon 0.44.2

Fixes from the first tests of the storage sorter:

- **Double chests open from either side.** The sorter used to aim at one fixed half of a double chest and gave up when the other half was in the way, so it turned its head for seconds, skipped containers (the overflow one too) and put everything back into the input. It now walks to the nearer half and clicks the half in sight; a click on either half counts. The farm and mining trips use the same fix.
- **No more clicks into item frames.** A container click whose line passes through an item frame is not made (it would turn the frame's item or put the held item in); the aim uses the chest's edges beside the frame instead.
- **No false "full".** The sorter reads your slots from the open window and only treats a container as full when nothing has moved for 1.5 seconds, so it no longer leaves after one stack.
- **Faster and non-stop.** Clicks come every 100 ms. **Watch** keeps a single input open and sorts new items the moment they arrive; with several inputs it goes round them every 5 seconds.

The Horizon package targets the official Nightly `8f83e5c`; the Sunrise package targets Nightly `9425651`.

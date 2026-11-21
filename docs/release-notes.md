# DraimHorizon 0.44.3

Fixes for the storage sorter (and every container trip):

- **Fast, steady aim at containers.** The head now turns straight to a clickable point of the chest at 45° a tick and clicks there, whether Human look is on or off. Before, it swung between the chest's middle and a corner, tried a click only every other tick and gave up on the chest after about a second, which is why the sorter could not reach its chests.
- **Clear messages.** When the sorter skips a container or cannot open the input, the chat says which one and what the aim met instead.

From 0.44.2: double chests open from either side, clicks never go through item frames, no false "full" after one stack, clicks every 100 ms, and **Watch** keeps a single input open and sorts new items at once.

The Horizon package targets the official Nightly `8f83e5c`; the Sunrise package targets Nightly `9425651`.

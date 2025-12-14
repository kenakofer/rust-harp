# Rust MIDI Harp

A low-latency, windowed MIDI controller application designed for Linux.

## Functionality
  * **Interaction**: Dragging the mouse cursor across a line triggers a MIDI Note On event. Using a wacom tablet with a vertically-bumped cover provides haptic feedback as if dragging across strings.
  * **Sound**: Acts as a virtual MIDI device (ALSA sequencer) named "Rust Harp Output".
      Connect this output to any DAW or synthesizer to produce sound.
  * **Rendering**: Visuals are functional, not fancy; in the long run, the goal is headless raspi or something.

# Rust MIDI Harp

A low-latency musical instrument, inspired by the autoharp, designed for Android and Linux. It's NOT well-tested on different phones.

## Functionality
  * **Interaction**: Swiping across a line plays a note. On linux, it's nice to use a wacom tablet with a vertically-bumped cover provides haptic feedback as if dragging across strings.
  * **Chords**: On android, chords can be changed with simple cardinal-direction gestures. Compose directions for more complicated chord variants. On linux, there's a hard-coded keyboard mapping.
  * **Sound**: On linux, can act as a virtual MIDI device (ALSA sequencer) named "Rust Harp Output". On both android and linux the app can make it's own very basic low-latency square-wave output too.
  * **Rendering**: Visuals are functional, not fancy.

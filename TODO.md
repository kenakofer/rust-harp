Raise Ceiling: +
Lower Floor:   -
Both:          *

General:
  - Adjust bottom note without changing key
  - Pulse sometimes gets removed from queue while held
  * Alt chording mode: 1-octave piano-style keyboard for note inclusion
    - Certain buttons like change key, pedal, pulse, etc. would still need dedicated buttons.
  + Pedal toggle/mod?

Android app:
* Prevent swiping off the left/right sides closing the app (forward/backward button)
* Add Pulse
* Make notes sound less derpy (attack envelope?)
* Dedupe rapid repeats
* Better audio out device
* Octave up/down split or modifier
- Add more state to display
* Larger screen (larger android device? iOS?)

Desktop app:
* Currently the top note is Ti when the bottom is Do. Can we fit one more?
- Add state to display
* Find large multi-touch trackpad

---

Codebase modularity refactors:
* Split `desktop_frontend.rs` into focused modules (window/event loop, audio routing, rendering, settings UI).
* Split `android_jni.rs` into focused modules (JNI exports thin layer + audio/render/input/settings helpers).
* Extract a shared strings renderer (used by both desktop + Android) and keep platform pixel-buffer plumbing separate.
* Split `app_state.rs` into smaller core modules (events/state/effects) and privatize internal fields currently marked TODO.
* Introduce an audio/output abstraction (trait) so `AppEffects` application is shared across platforms.
* Clean up module boundaries: remove/rename `adapter.rs` indirection; make `ui_adapter.rs` clearly platform-specific.


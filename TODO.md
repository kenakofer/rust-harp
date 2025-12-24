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
* Chords are slow to register (relative to speed of typing)
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

Audio backend selection (Android + Desktop)

---

## App chord controls rework (tap + directional swipe wheel)

Goal: remove modifier buttons and replace with a press-and-swipe radial chooser per chord button, plus a double-tap M/m toggle.

### 0) Behavior spec (to keep us honest)
- Remove modifier button row entirely.
- Chord-button multi-press no longer implies Maj/Min7 behavior.
- On pointer-down on a chord button: show an 8-sector wheel around that button.
- As finger moves: highlight the nearest of 8 directions and **apply/preview that chord+modifier immediately** (while still holding).
  - On release: keep the last-applied modifier active (no extra change needed).
- If press→release with no swipe, then a quick second press (double-tap): apply M/m switch.
- Provide a short-lived visual state after first tap to indicate double-tap window (e.g. button label changes briefly).

### 1) Data model & mapping (Rust, shared by all frontends)
- Add `ChordWheelChoice` enum with 8 variants (N, NE, E, SE, S, SW, W, NW) OR a `WheelDir8` + mapping tables.
- Add `ChordModifierPreset` struct describing the exact modifier set needed (Maj/Min, add2, sus4, add7/m7/M7, etc.).
- Implement two mapping tables:
  - `wheel_mapping_for_major_degree(degree) -> [ChordModifierPreset; 8]`
  - `wheel_mapping_for_minor_or_dim_degree(degree) -> [ChordModifierPreset; 8]`
  (use the user-provided clockwise lists).
- Expose a single API used by the app UI:
  - `fn apply_chord_wheel_choice(app: &mut AppState, base_button: UiButton, dir: WheelDir8)`

### 2) Gesture recognition (Android)
- For each chord button press, track:
  - pointerId, down position, down time, active base button.
  - current selected wheel dir (optional).
- Define constants:
  - `WHEEL_DEADZONE_PX` (movement before selection is considered)
  - `DOUBLE_TAP_MS`
- Compute direction as `atan2(dy, dx)` quantized to 8 sectors.
- Display wheel overlay while pointer is down; update highlight on move.
- On move:
  - If moved past deadzone and sector changes: apply that sector immediately.
- On up:
  - If we never moved past deadzone: arm double-tap window; if second tap arrives within `DOUBLE_TAP_MS` on same base chord button: apply M/m toggle.
  - If we did apply a sector while holding: do nothing special (the modifier is already active).

### 3) Rendering/UI (Android)
- Remove modifier grid views.
- Implement a lightweight wheel overlay in the existing bitmap renderer:
  - Draw 8 wedge highlights around the pressed chord button rect.
  - Draw labels in each wedge (use short labels: `^7`, `^9`, `+2`, `sus`, `^M7`, etc.).
  - When a wedge is selected, brighten that wedge.
- Double-tap armed state: temporarily swap the chord button label (or invert color) until timeout.

### 4) State sync (keyboard ↔ onscreen)
- Keyboard chord selection should still update which base chord is “active” (button highlight).
- When wheel choice applies a modified chord, update button highlight as usual (no separate modifier buttons).

### 5) Tests
- Add pure Rust tests for:
  - direction quantization (edge angles)
  - mapping tables for major/minor degrees
  - double-tap state machine (no-swipe tap → armed → second tap)
- Add Android unit tests for the gesture state machine (host tests) without needing emulator.

### 6) Desktop parity (later)
- Decide whether to also support wheel-on-click in desktop, or keep existing keyboard-driven modifiers only.
  - If we want parity: implement same wheel overlay + mouse-drag selection.

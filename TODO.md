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

<<<<<<< Updated upstream
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
=======

Very nice! Lets do a more sizeable control rework for the app. The goal is for the app to not have modifier buttons, and instead use a tap + directional swipe to indicate modifiers:
1. The row of modifier buttons in the app goes away.
2. On the app: Simultaneously pressing neighboring chord buttons no longer makes it a major-minor 7 chord.
3. Now, when pressing down on a chord button, a wheel of choices shows around the button. Swiping in one of the eight directions makes that the active chord with modifiers. In the following list, "^" indicates superscript.
  - From the top, in clockwise order for major chords e.g. the IV chord
    - Major +m7 "IV^7"
    - Major +m7 +M2 "IV^9"
    - Add 2 "IV+2"
    - Minor +m7 +M2 "iv^9"
    - Minor +m7 "iv^7"
    - Minor +M7 "iv^M7"
    - sus4 -3 +4 "IVsus"
    - Major +M7 "IV^M7"
  - From the top, in clockwise order for minor/diminished chords e.g. the iii chord
    - Major +m7 "III^7"
    - Major +m7 +M2 "III^9"
    - Add 2 "iii+2"
    - Minor +m7 +M2 "iii^9"
    - Minor +m7 "iii^7"
    - Minor +M7 "iii^M7"
    - sus4 -3 +4 "iiisus"
    - Major +M7 "III^M7"
- We'll add a double-tap gesture too. If the user presses, doesn't swipe, releases, then presses again, apply the M/m switch. This can be visually indicated by changing the button label for a short period after a press/release.
- We'll want nice visual indicators that highlight sectors of the circle as the user holds to select.
- Make a plan for implementation, and write it to the TODO.md file.


Wow, that works really well! Some style tweaks after the first test:
  1. The wheel is drawing below the chord buttons, but should go on top.
  2. The wheel needs to be rotated half a button counter-clockwise; currently the +m7 is up and bit right, but it should be cardinally straight up.
  3. The chord buttons should all have a left and bottom margin (perhaps 30px) to allow for down and left swipes.
  4. After a tap, we're currently setting the label to "M/m", but it would be better to set it to the correct roman numeral/chord name: "Am" -> "A", "ii" -> "II", "VIIb" -> "viib", etc.


I'm not sure if this is an android issue, or something in our code, but when I double tap a chord button and it does the m/M (which is correct), the button then stays "highlighted" as if it's still being pressed (not correct). Then, while that chord button is thus hightlighted, single taps on a different chord button fail to change the chord. Do you know what's going on with that?


That works well! Next, I'd like to adjust the app's note-off events to only fire when a chord-button press has been released and the double tap window has expired, whichever is later. That way we don't fire several undesired note-offs while getting to the desired chord.


Cool! Next, I'd like to add some addtional syth sounds. Triangle and sawtooth would be cool. It would be nice to add some

Perfect! Next, I want to add better visuals in the app for when a note is actively playing. To keep it simple, how about for note strikes we flash and fade a white rectangle on the note strike region. For strummed notes, lets draw the string with a wider rectangle then decrease the width back to the original.

Neat! Next, I want to add a harmonic_minor() chord builder to use in the second row. Currently we're checking if the major scale on each root contains the root. Lets update to, for each root, check the harmonic minor (0, 2, 3, 5, 7, 8, 11) first, then the major.


>>>>>>> Stashed changes

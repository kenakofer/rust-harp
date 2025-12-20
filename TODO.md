## TODO
  - rearrange mods/4 doesn't remove 3
  - Pedal toggle/mod?
  - Phone app version?
    - [x] Extract platform-agnostic core (AppState/chords/notes + core events/effects) into lib.rs
    - [ ] Add UI frontends (winit-desktop, android-touch) that emit core input events
      - [x] winit-desktop frontend module
      - [ ] android-touch frontend module
        - [x] core touch tracker (multi-pointer crossings)
    - [ ] Add output backends (midi today; android audio+dsp + haptics later) consuming core effects
      - [x] midi backend (velocity split + midir sender)
      - [ ] android audio + haptics backend
    - Implement plucked-string synth (Karplus–Strong / modal) + rate-limited haptic ticks on crossings
    - [ ] Build Android APKs in GitHub Actions (publish as artifacts) to avoid installing SDK/NDK/Gradle locally
      - [x] Minimal Gradle Android project + JNI smoke test
      - [ ] GitHub Actions workflow builds arm64 debug APK artifact (passing)
  - Check bass balance on keychange
  - Explore additive chords (I + vi = vi7, I + iii = Imaj7)
  - Currently the top note is Ti when the bottom is Do. Can we fit one more?
  - NUM_STRINGS is really not that useful, but its used everywhere, even where we should use NUM_NOTES instead. How to find NUM_NOTES?
  - Notes never getting stopped
  - Pulse can get removed from queue while held
  - Need to create new event for strum note crossings to pass to app_state for filtering.
    - This will let us keep track of notes on and off.

## Android MVP: hook core AppState + audio
Goal: ship an APK where a **Bluetooth keyboard** changes chords using the same mapping as desktop, and when core emits `AppEffects.play_notes` we play a **1s square-wave** tone at the correct pitch.

### 1) Share a single key mapping across desktop + Android
- [x] Extracted into `src/input_map.rs` and desktop now adapts winit -> `UiKey`.
- Extract the current desktop mapping in `src/ui_adapter.rs` into a core module (e.g. `src/input_map.rs`) that maps:
  - `KeyState` + a *simple key representation* (e.g. `char` or an enum) -> `app_state::KeyEvent`
- Desktop keeps using winit by adapting winit’s `KeyEvent` -> (char/enum) -> `KeyEvent`.
- Android uses the same mapping by adapting Android’s `android.view.KeyEvent` -> (char/enum) -> `KeyEvent`.
- Add unit tests that assert the mapping table is stable (e.g. 'a' -> VIIB, 's' -> IV, 'd' -> I, 'f' -> V, etc).

### 2) Add an Android “frontend” Rust object that owns Engine
- [x] Added `AndroidFrontend` + JNI create/destroy handle lifecycle.
- Create `src/android_frontend.rs` (cfg android) that contains something like:
  - `struct AndroidFrontend { engine: Engine }`
- Expose JNI lifecycle:
  - `rustCreateFrontend() -> jlong` (boxed pointer handle)
  - `rustDestroyFrontend(handle)`

### 3) Keyboard input plumbing (Bluetooth keyboard)
- [x] `MainActivity.dispatchKeyEvent` wired to JNI + shared `input_map`.
- In `MainActivity`, override `dispatchKeyEvent(android.view.KeyEvent e)`.
- Convert Android key events to our shared key representation:
  - Prefer `e.getKeyCode()` for non-printable keys (CTRL, TAB), and/or `e.getUnicodeChar()` for letter/number keys.
  - Determine `Pressed/Released` via `e.getAction()`.
- JNI call: `rustHandleAndroidKey(handle, keyCode, unicodeChar, isDown)`.
  - Rust converts that into `app_state::KeyEvent` using the shared mapping and calls `engine.handle_event(..)`.

### 4) Make rendering reflect the active chord from AppState
- [x] Android rendering reads `Engine.active_chord()` and colors root/chord/non-chord.
- Replace the current “always root_pc=0” highlighting in `rustRenderStrings`.
- Use `engine.active_chord()` and mirror the desktop logic:
  - draw root notes in red (`Chord::has_root`)
  - draw chord tones in white (`Chord::contains`)
  - non-chord strings dim/gray
- Trigger re-render when `AppEffects.redraw == true`.
- Avoid per-frame allocations on Android:
  - Keep a single `Bitmap` and `int[]` pixel buffer sized to the screen.
  - Only redraw on input events / state changes.

### 5) Square-wave audio output for AppEffects.play_notes
- [x] Upgraded: Rust streaming synth (decaying square waves) + Java `AudioTrack` loop calling `rustFillAudio`.
- Add a tiny Android audio backend on the Java side (simplest/fastest path):
  - `TonePlayer` using `AudioTrack` (PCM 16-bit, e.g. 48kHz).
  - Generate a 1-second square wave buffer for a given frequency and amplitude.
  - Play on a background thread to avoid blocking UI.
- When Rust returns `AppEffects.play_notes`, for each note:
  - Convert to pitch:
    - Option A (simple): Rust returns MIDI note number (use the same `MIDI_BASE_TRANSPOSE` as desktop, currently 36) and Java converts to Hz via `freq = 440 * 2^((midi-69)/12)`.
    - Option B: Rust returns Hz directly.
  - Convert `NoteVolume` (0..127) to amplitude scaling.
  - Call `TonePlayer.playSquare(freqHz, amplitude, 1.0s)`.
- Initial polyphony strategy (keep it simple):
  - Either mix multiple square waves into one buffer (clamp), or just play the loudest note.

### 6) CI / debugging hooks
- Extend the workflow to also upload `logcat` on failure (optional).
- Add a debug overlay (optional) that prints current active chord + last key received.

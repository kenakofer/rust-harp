Raise Ceiling: +
Lower Floor:   -
Both:          *

General:
  + Explore additive chords (I + vi = vi7, I + iii = Imaj7)
  * Currently the top note is Ti when the bottom is Do. Can we fit one more?
  - Notes are never getting stopped
  - Pulse sometimes gets removed from queue while held
  * Alt chording mode: 1-octave piano-style keyboard for note inclusion
    - Certain buttons like change key, pedal, pulse, etc. would still need dedicated buttons.
  * Press plays closest note (in addition to strum)
    - De-duping might be hard. Not impossible though. We just want to make sure if the press happens left of center on the key, the next swipe right over the center is ignored.
  + Pedal toggle/mod?

Android app:
* Prevent swiping off the left/right sides closing the app (forward/backward button)
* Chords are slow to register (relative to speed of typing)
* Add Pulse
* Raise max voices to prevent cutoffs
* Make notes sound less derpy (attack envelope?)
* Dedupe rapid repeats
* Stop notes outside chord
* Better audio out device
* Octave up/down split or modifier
- Add state to display
* Larger screen (larger android device)
- On screen buttons so BT keyboard not necessary.
- Add state to display

Desktop app:
- Add state to display
* Find large multi-touch trackpad

Audio backend selection (Android + Desktop)

Goal
- Add a user-selectable "Audio backend" setting.
- Android: choose between AAudio (preferred) and AudioTrack fallback.
- Desktop (Linux): choose between MIDI (current) and built-in Rust synth output (direct audio).

Terminology
- "Audio backend" = how we turn AppState/AppEffects play/stop notes into actual sound output.

Plan (multi-step)
1) Define a shared backend enum + settings plumbing
   - Add/extend a shared enum (e.g. AudioBackend { AAudio, AudioTrack } on Android; { Midi, Synth } on desktop).
   - Put the enum in a shared module that both frontends can use (alongside existing settings types).
   - Add serialization/persistence:
     - Android: SharedPreferences
     - Desktop: same config mechanism used by desktop settings (or minimal file-based if none exists yet).

2) Create a small cross-frontend Rust interface
   - Define a trait like:
     - start(sample_rate/…)
     - stop()
     - handle_effects(play_notes + stop_notes)
   - Keep the trait in Rust (core/shared) and make frontends own an implementation.

3) Android: wire the setting to the audio engine
   - In MainActivity/settings panel: dropdown "Audio" with { "AAudio", "AudioTrack" }.
   - When changed:
     - stop current engine
     - construct + start the selected engine
     - continue feeding it AppEffects
   - Keep AAudio as default; allow fallback if AAudio fails to open.

4) Desktop Linux: add "Synth" output via ALSA (recommend using cpal)
   - Use the existing Rust synth voice engine, but drive it with a real audio device.
   - Use the `cpal` crate:
     - On Linux, cpal will use ALSA by default (or PipeWire/JACK depending on system).
     - This avoids writing raw ALSA bindings.
   - Implement DesktopAudioBackend::Synth:
     - spawn audio stream callback
     - feed note events via lock-free-ish channel (ring buffer) to avoid xruns.
   - Implement DesktopAudioBackend::Midi by reusing current MIDI path.

5) Desktop UI
   - Add a dropdown "Audio" with { "MIDI", "Synth" }.
   - Switching backends should stop the old backend cleanly before starting the new.

6) Testing approach (what we can unit test)
   - Unit tests for:
     - backend selection parsing/serialization
     - backend switch logic calls stop/start in the right order (use a fake backend implementation)
     - synth event queue correctness (notes played/stopped, de-dupe rules)
   - We won’t be able to fully unit test cpal/ALSA devices in CI, but we can keep the device-specific layer thin.

Milestones
- M1: Shared enum + settings UI hooks (no behavior change yet).
- M2: Android switching works (AAudio <-> AudioTrack).
- M3: Desktop Synth backend producing audio via cpal.
- M4: Desktop switching works (MIDI <-> Synth) + basic unit tests.

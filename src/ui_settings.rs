#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiAudioBackend {
    // Android
    AAudio,
    AudioTrack,
    // Desktop
    Midi,
    Synth,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiSettings {
    pub show_note_names: bool,
    pub play_on_tap: bool,
    pub show_roman_chords: bool,
    // Android-only: whether on-screen chord buttons are visible.
    pub show_chord_buttons: bool,

    // Selected audio output backend (UI-facing selection; not all backends are implemented on all platforms yet).
    pub audio_backend: UiAudioBackend,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            show_note_names: false,
            play_on_tap: true,
            show_roman_chords: true,
            show_chord_buttons: true,
            audio_backend: UiAudioBackend::Midi,
        }
    }
}

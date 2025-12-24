#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiAudioBackend {
    // Android
    AAudio,
    AudioTrack,
    // Desktop
    Midi,
    Synth,
}

impl UiAudioBackend {
    pub fn cycle_desktop(self) -> Self {
        match self {
            UiAudioBackend::Midi => UiAudioBackend::Synth,
            UiAudioBackend::Synth => UiAudioBackend::Midi,
            _ => UiAudioBackend::Midi,
        }
    }

    pub fn cycle_android(self) -> Self {
        match self {
            UiAudioBackend::AAudio => UiAudioBackend::AudioTrack,
            UiAudioBackend::AudioTrack => UiAudioBackend::AAudio,
            _ => UiAudioBackend::AAudio,
        }
    }
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

    // Synth tuning reference (A4) in Hz.
    pub a4_tuning_hz: u16,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            show_note_names: false,
            play_on_tap: true,
            show_roman_chords: true,
            show_chord_buttons: true,
            audio_backend: UiAudioBackend::Midi,
            a4_tuning_hz: 440,
        }
    }
}

#[cfg(feature = "desktop")]
fn encode_settings(s: &UiSettings) -> String {
    let backend = match s.audio_backend {
        UiAudioBackend::Synth => "synth",
        _ => "midi",
    };

    format!(
        "show_note_names={}\nplay_on_tap={}\nshow_roman_chords={}\naudio_backend={}\na4_tuning_hz={}\n",
        s.show_note_names,
        s.play_on_tap,
        s.show_roman_chords,
        backend,
        s.a4_tuning_hz
    )
}

#[cfg(feature = "desktop")]
fn decode_settings(input: &str) -> UiSettings {
    let mut s = UiSettings::default();

    for line in input.lines() {
        let Some((k, v)) = line.split_once('=') else { continue };
        match k.trim() {
            "show_note_names" => s.show_note_names = v.trim() == "true",
            "play_on_tap" => s.play_on_tap = v.trim() != "false",
            "show_roman_chords" => s.show_roman_chords = v.trim() != "false",
            "audio_backend" => {
                s.audio_backend = match v.trim() {
                    "synth" => UiAudioBackend::Synth,
                    _ => UiAudioBackend::Midi,
                }
            }
            "a4_tuning_hz" => {
                if let Ok(hz) = v.trim().parse::<u16>() {
                    s.a4_tuning_hz = hz.clamp(430, 450);
                }
            }
            _ => {}
        }
    }

    s
}

#[cfg(feature = "desktop")]
fn desktop_settings_path() -> Option<std::path::PathBuf> {
    use std::path::PathBuf;

    #[cfg(windows)]
    if let Ok(appdata) = std::env::var("APPDATA") {
        return Some(PathBuf::from(appdata).join("rust-harp").join("settings.txt"));
    }

    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(xdg).join("rust-harp").join("settings.txt"));
    }

    if let Ok(home) = std::env::var("HOME") {
        return Some(PathBuf::from(home).join(".config").join("rust-harp").join("settings.txt"));
    }

    None
}

#[cfg(feature = "desktop")]
pub fn load_desktop_settings() -> UiSettings {
    use std::fs;

    let Some(path) = desktop_settings_path() else {
        return UiSettings::default();
    };

    match fs::read_to_string(&path) {
        Ok(s) => decode_settings(&s),
        Err(_) => UiSettings::default(),
    }
}

#[cfg(feature = "desktop")]
pub fn save_desktop_settings(s: &UiSettings) {
    use std::fs;

    let Some(path) = desktop_settings_path() else {
        return;
    };

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let _ = fs::write(path, encode_settings(s));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_desktop_audio_backend() {
        assert_eq!(UiAudioBackend::Midi.cycle_desktop(), UiAudioBackend::Synth);
        assert_eq!(UiAudioBackend::Synth.cycle_desktop(), UiAudioBackend::Midi);
        // Non-desktop values fall back to a sensible default.
        assert_eq!(UiAudioBackend::AAudio.cycle_desktop(), UiAudioBackend::Midi);
    }

    #[test]
    fn cycle_android_audio_backend() {
        assert_eq!(UiAudioBackend::AAudio.cycle_android(), UiAudioBackend::AudioTrack);
        assert_eq!(UiAudioBackend::AudioTrack.cycle_android(), UiAudioBackend::AAudio);
        // Non-android values fall back to a sensible default.
        assert_eq!(UiAudioBackend::Midi.cycle_android(), UiAudioBackend::AAudio);
    }

    #[cfg(feature = "desktop")]
    #[test]
    fn desktop_settings_roundtrip() {
        let s = UiSettings {
            show_note_names: true,
            play_on_tap: false,
            show_roman_chords: false,
            show_chord_buttons: true,
            audio_backend: UiAudioBackend::Synth,
            a4_tuning_hz: 432,
        };

        let enc = super::encode_settings(&s);
        let dec = super::decode_settings(&enc);
        assert_eq!(dec.show_note_names, true);
        assert_eq!(dec.play_on_tap, false);
        assert_eq!(dec.show_roman_chords, false);
        assert_eq!(dec.audio_backend, UiAudioBackend::Synth);
        assert_eq!(dec.a4_tuning_hz, 432);
    }
}

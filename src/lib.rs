pub mod app_state;
pub mod chord;
pub mod engine;
pub mod input_map;
pub mod layout;
pub mod notes;
pub mod output_midi;
#[cfg(feature = "midi")]
pub mod output_midir;
pub mod rows;
pub mod strum;
pub mod touch;
pub mod ui_events;

#[cfg(feature = "android")]
pub mod android_audio;

#[cfg(feature = "android")]
pub mod android_frontend;

#[cfg(all(target_os = "android", feature = "android"))]
pub mod android_aaudio;

#[cfg(all(target_os = "android", feature = "android"))]
pub mod android_jni;

#[cfg(all(feature = "desktop", feature = "midi"))]
pub mod desktop_frontend;

#[cfg(feature = "desktop")]
pub mod ui_adapter;

#[cfg(feature = "desktop")]
pub mod adapter;

pub mod app_state;
pub mod chord;
pub mod engine;
pub mod notes;
pub mod output_midi;
pub mod strum;

#[cfg(all(feature = "desktop", feature = "midi"))]
pub mod desktop_frontend;

#[cfg(feature = "desktop")]
pub mod ui_adapter;

#[cfg(feature = "desktop")]
pub mod adapter;

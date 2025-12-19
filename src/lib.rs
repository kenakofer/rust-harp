pub mod app_state;
pub mod chord;
pub mod notes;
pub mod output_midi;

#[cfg(feature = "desktop")]
pub mod ui_adapter;

#[cfg(feature = "desktop")]
pub mod adapter;

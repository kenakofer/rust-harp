mod audio;
mod render;

use crate::notes::UnmidiNote;
use crate::output_midir::MidiBackend;
use crate::touch::{PointerId, TouchEvent, TouchPhase};
use crate::ui_adapter::{self};
use crate::ui_events::{UiEvent, UiSession};
use crate::ui_settings::UiAudioBackend;

#[cfg(any(target_os = "linux", target_os = "macos"))]
use midir::os::unix::VirtualOutput;

use midir::{MidiOutput, MidiOutputConnection};
use softbuffer::{Context, Surface};
use std::error::Error;
use std::num::NonZeroU32;
use std::rc::Rc;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use self::audio::{
    process_app_effects, DesktopAudio, MICRO_CHANNEL, MICRO_PROGRAM, MIDI_BASE_TRANSPOSE,
};
use self::render::{draw_strings, hit_rect, hit_settings_rows, settings_layout, SettingsAction};

const MAIN_PROGRAM: u8 = 25; // Steel String Guitar (zero-based)
const MAIN_CHANNEL: u8 = 0;
const BASS_PROGRAM: u8 = 26;
const BASS_CHANNEL: u8 = 2;

pub fn run() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // 1. Setup MIDI Output
    let midi_out = MidiOutput::new("Rust Harp Client")?;
    let mut conn_out: Option<MidiOutputConnection> = None;

    // Attempt to create virtual port on systems where that exists
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    match midi_out.create_virtual("Rust Harp Output") {
        Ok(conn) => {
            log::info!("Created virtual MIDI port: 'Rust Harp Output'");
            conn_out = Some(conn);
        }
        Err(_) => {
            // Fallback for non-ALSA environments or errors
            let midi_out = MidiOutput::new("Rust Harp Client")?;
            let ports = midi_out.ports();
            if let Some(port) = ports.first() {
                log::info!(
                    "Virtual port failed. Connecting to first available hardware port: {}",
                    midi_out.port_name(port)?
                );
                conn_out = Some(midi_out.connect(port, "Rust Harp Connection")?);
            } else {
                eprintln!("Warning: No MIDI ports found. Application will emit no sound.");
            }
        }
    }

    #[cfg(any(target_os = "windows"))]
    if let Some(port) = midi_out.ports().first() {
        log::info!(
            "Connecting to hardware MIDI port: {}",
            midi_out.port_name(port)?
        );
        conn_out = Some(midi_out.connect(port, "Rust Harp Connection")?);
    } else {
        eprintln!("Warning: No MIDI ports found. Application will emit no sound.");
    }

    let mut audio = DesktopAudio::new(MidiBackend::new(conn_out, MAIN_CHANNEL, BASS_CHANNEL));

    // If we have a connection, set the instruments
    if let Some(conn) = audio.midi_mut().conn_mut() {
        let _ = conn.send(&[0xC0 | MAIN_CHANNEL, MAIN_PROGRAM]);
        let _ = conn.send(&[0xC0 | BASS_CHANNEL, BASS_PROGRAM]);
        let _ = conn.send(&[0xC0 | MICRO_CHANNEL, MICRO_PROGRAM]);
    }

    // Setup Window
    let event_loop = EventLoop::new()?;
    let window = Rc::new(
        WindowBuilder::new()
            .with_title("Rust MIDI Harp")
            .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0))
            .build(&event_loop)?,
    );

    // Setup Graphics Context and UX state
    let context = Context::new(window.clone()).expect("Failed to create graphics context");
    let mut surface = Surface::new(&context, window.clone()).expect("Failed to create surface");
    let mut prev_pos: Option<(f32, f32)> = None;
    let mut is_mouse_down = false;
    let mut positions_cache = crate::layout::NotePositionsCache::default();

    // App State
    let mut ui = UiSession::new();
    let mut settings = crate::ui_settings::load_desktop_settings();
    ui.set_play_on_tap(settings.play_on_tap);
    audio.set_a4_tuning_hz(settings.a4_tuning_hz);
    let mut show_settings = false;

    // Precompute once so non-resize events (e.g. key presses) still have positions.
    let _ = positions_cache.desktop(window.inner_size().width as f32);

    // 4. Run Event Loop
    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Wait);

        match event {
            Event::WindowEvent { window_id, event } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => {
                    audio.stop_all_notes_all_backends(ui.engine().active_notes());
                    elwt.exit();
                }

                WindowEvent::KeyboardInput { event, .. } => {
                    if let Some(ue) = ui_adapter::ui_event_from_winit(&event) {
                        let positions =
                            positions_cache.desktop(window.inner_size().width.max(1) as f32);
                        let out = ui.handle_with_settings(ue, positions, &settings);
                        let _ = process_app_effects(
                            out.effects,
                            &mut audio,
                            settings.audio_backend,
                            Some(window.as_ref()),
                        );
                    }
                }

                WindowEvent::Resized(physical_size) => {
                    surface
                        .resize(
                            NonZeroU32::new(physical_size.width).unwrap(),
                            NonZeroU32::new(physical_size.height).unwrap(),
                        )
                        .unwrap();

                    let window_width = physical_size.width as f32;
                    let positions = positions_cache.desktop(window_width.max(1.0));

                    let row_chords = ui.engine().row_chords();
                    draw_strings(
                        &mut surface,
                        physical_size.width,
                        physical_size.height,
                        ui.engine().row_specs(),
                        &row_chords,
                        positions,
                        settings.show_note_names,
                        ui.engine().transpose().wrap_to_octave(),
                        show_settings,
                        &settings,
                    );
                }

                WindowEvent::MouseInput { state, button, .. } => {
                    if button == winit::event::MouseButton::Left {
                        let pressed = state == winit::event::ElementState::Pressed;

                        let Some((x, y)) = prev_pos else {
                            return;
                        };

                        // Gear icon + settings panel (desktop only).
                        if pressed {
                            let (gear, panel, rows) = settings_layout(
                                window.inner_size().width,
                                window.inner_size().height,
                            );
                            if hit_rect(x, y, gear) {
                                show_settings = !show_settings;
                                window.request_redraw();
                                return;
                            }
                            if show_settings {
                                if hit_rect(x, y, panel) {
                                    if let Some(action) = hit_settings_rows(x, y, rows) {
                                        match action {
                                            SettingsAction::TogglePlayOnTap => {
                                                settings.play_on_tap = !settings.play_on_tap;
                                                ui.set_play_on_tap(settings.play_on_tap);
                                                crate::ui_settings::save_desktop_settings(
                                                    &settings,
                                                );
                                            }
                                            SettingsAction::ToggleShowNoteNames => {
                                                settings.show_note_names =
                                                    !settings.show_note_names;
                                                crate::ui_settings::save_desktop_settings(
                                                    &settings,
                                                );
                                            }
                                            SettingsAction::ToggleShowRomanChords => {
                                                settings.show_roman_chords =
                                                    !settings.show_roman_chords;
                                                crate::ui_settings::save_desktop_settings(
                                                    &settings,
                                                );
                                            }
                                            SettingsAction::CycleAudioBackend => {
                                                // Stop currently playing notes on the *current* backend so we don't leave
                                                // hanging notes behind when switching.
                                                let notes: Vec<UnmidiNote> =
                                                    ui.engine().active_notes().collect();
                                                for n in notes {
                                                    audio.stop_note(
                                                        settings.audio_backend,
                                                        MIDI_BASE_TRANSPOSE + n,
                                                    );
                                                }

                                                settings.audio_backend =
                                                    settings.audio_backend.cycle_desktop();

                                                #[cfg(feature = "synth")]
                                                if settings.audio_backend == UiAudioBackend::Synth
                                                    && !audio.has_synth()
                                                {
                                                    settings.audio_backend = UiAudioBackend::Midi;
                                                }

                                                audio.set_a4_tuning_hz(settings.a4_tuning_hz);
                                                crate::ui_settings::save_desktop_settings(
                                                    &settings,
                                                );
                                            }
                                            SettingsAction::SetA4Tuning(hz) => {
                                                settings.a4_tuning_hz = hz.clamp(430, 450);
                                                audio.set_a4_tuning_hz(settings.a4_tuning_hz);
                                                crate::ui_settings::save_desktop_settings(
                                                    &settings,
                                                );
                                            }
                                        }
                                    }
                                    window.request_redraw();
                                    return;
                                }
                                // Click outside closes.
                                show_settings = false;
                                window.request_redraw();
                            }
                        }

                        is_mouse_down = pressed;

                        let phase = if pressed {
                            TouchPhase::Down
                        } else {
                            TouchPhase::Up
                        };
                        let h = window.inner_size().height.max(1) as f32;
                        let positions =
                            positions_cache.desktop(window.inner_size().width.max(1) as f32);
                        let out = ui.handle_with_settings(
                            UiEvent::Touch(TouchEvent {
                                id: PointerId(0),
                                phase,
                                x,
                                y_norm: (y / h).clamp(0.0, 1.0),
                                pressure: 1.0,
                            }),
                            positions,
                            &settings,
                        );
                        let _ = process_app_effects(
                            out.effects,
                            &mut audio,
                            settings.audio_backend,
                            Some(window.as_ref()),
                        );
                    }
                }

                WindowEvent::CursorMoved { position, .. } => {
                    let curr_x = position.x as f32;
                    let curr_y = position.y as f32;

                    if is_mouse_down {
                        let h = window.inner_size().height.max(1) as f32;
                        let positions =
                            positions_cache.desktop(window.inner_size().width.max(1) as f32);
                        let out = ui.handle_with_settings(
                            UiEvent::Touch(TouchEvent {
                                id: PointerId(0),
                                phase: TouchPhase::Move,
                                x: curr_x,
                                y_norm: (curr_y / h).clamp(0.0, 1.0),
                                pressure: 1.0,
                            }),
                            positions,
                            &settings,
                        );
                        let _ = process_app_effects(
                            out.effects,
                            &mut audio,
                            settings.audio_backend,
                            Some(window.as_ref()),
                        );
                    }

                    prev_pos = Some((curr_x, curr_y));
                }

                WindowEvent::RedrawRequested => {
                    let size = window.inner_size();
                    let positions = positions_cache.desktop(size.width.max(1) as f32);
                    let row_chords = ui.engine().row_chords();
                    draw_strings(
                        &mut surface,
                        size.width,
                        size.height,
                        ui.engine().row_specs(),
                        &row_chords,
                        positions,
                        settings.show_note_names,
                        ui.engine().transpose().wrap_to_octave(),
                        show_settings,
                        &settings,
                    );
                }

                _ => {}
            },
            _ => {}
        }
    })?;

    Ok(())
}

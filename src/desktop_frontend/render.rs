use crate::chord::Chord;
use crate::ui_settings::UiAudioBackend;

use softbuffer::Surface;
use std::rc::Rc;
use winit::window::Window;

#[derive(Clone, Copy)]
pub(crate) struct RectI32 {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) w: i32,
    pub(crate) h: i32,
}

pub(crate) fn hit_rect(x: f32, y: f32, r: RectI32) -> bool {
    let (x, y) = (x.round() as i32, y.round() as i32);
    x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum SettingsAction {
    TogglePlayOnTap,
    ToggleShowNoteNames,
    ToggleShowRomanChords,
    CycleAudioBackend,
    SetA4Tuning(u16),
}

pub(crate) fn settings_layout(width: u32, _height: u32) -> (RectI32, RectI32, [RectI32; 5]) {
    // Fixed-size pixel UI; good enough for now.
    let gear = RectI32 {
        x: width as i32 - 44,
        y: 8,
        w: 36,
        h: 18,
    };

    let row_h = 20;

    let panel = RectI32 {
        x: width as i32 - 170,
        y: 30,
        w: 162,
        h: 5 * row_h,
    };

    let rows = [
        RectI32 {
            x: panel.x,
            y: panel.y,
            w: panel.w,
            h: row_h,
        },
        RectI32 {
            x: panel.x,
            y: panel.y + row_h,
            w: panel.w,
            h: row_h,
        },
        RectI32 {
            x: panel.x,
            y: panel.y + 2 * row_h,
            w: panel.w,
            h: row_h,
        },
        RectI32 {
            x: panel.x,
            y: panel.y + 3 * row_h,
            w: panel.w,
            h: row_h,
        },
        RectI32 {
            x: panel.x,
            y: panel.y + 4 * row_h,
            w: panel.w,
            h: row_h,
        },
    ];

    (gear, panel, rows)
}

pub(crate) fn hit_settings_rows(x: f32, y: f32, rows: [RectI32; 5]) -> Option<SettingsAction> {
    if hit_rect(x, y, rows[0]) {
        return Some(SettingsAction::TogglePlayOnTap);
    }
    if hit_rect(x, y, rows[1]) {
        return Some(SettingsAction::ToggleShowNoteNames);
    }
    if hit_rect(x, y, rows[2]) {
        return Some(SettingsAction::ToggleShowRomanChords);
    }
    if hit_rect(x, y, rows[3]) {
        return Some(SettingsAction::CycleAudioBackend);
    }
    if hit_rect(x, y, rows[4]) {
        // Map x position to 430..450.
        let x0 = (rows[4].x + 60) as f32;
        let x1 = (rows[4].x + rows[4].w - 6) as f32;
        let t = ((x - x0) / (x1 - x0)).clamp(0.0, 1.0);
        let hz = (430.0 + t * 20.0).round() as u16;
        return Some(SettingsAction::SetA4Tuning(hz));
    }
    None
}

pub(crate) fn draw_strings(
    surface: &mut Surface<Rc<Window>, Rc<Window>>,
    width: u32,
    height: u32,
    top_chord: Option<Chord>,
    bottom_chord: Chord,
    positions: &[f32],
    show_note_names: bool,
    transpose_pc: i16,
    show_settings: bool,
    settings: &crate::ui_settings::UiSettings,
) {
    let mut buffer = surface.buffer_mut().unwrap();
    buffer.fill(0);

    let (top_end, mid_end) = crate::render_shared::row_band_bounds(height as usize);

    let style = crate::render_best::BestStyle {
        root_prio: 2,
        root_color: 0x00FF0000,
        chord_prio: 1,
        chord_color: 0x00FFFFFF,
        inactive_prio: 0,
        inactive_color: 0,
    };

    let (top_prio, top_color, top_pc) = crate::render_best::compute_best_per_x(
        width as usize,
        positions,
        top_chord,
        transpose_pc,
        style,
        false,
    );
    let (mid_prio, mid_color, mid_pc) = crate::render_best::compute_best_per_x(
        width as usize,
        positions,
        Some(bottom_chord),
        transpose_pc,
        style,
        false,
    );
    let (bot_prio, bot_color, _bot_pc) = crate::render_best::compute_best_per_x(
        width as usize,
        positions,
        Some(bottom_chord.invert()),
        transpose_pc,
        style,
        false,
    );

    crate::render_shared::fill_string_bands(
        &mut buffer,
        width as usize,
        height as usize,
        top_end,
        mid_end,
        &top_prio,
        &top_color,
        &mid_prio,
        &mid_color,
        &bot_prio,
        &bot_color,
    );

    if show_note_names {
        crate::render_shared::draw_note_name_labels(
            &top_prio,
            &top_pc,
            &top_color,
            1,
            transpose_pc,
            4,
            2,
            |x, y, text, color| {
                crate::pixel_font::draw_text_u32(
                    &mut buffer,
                    width as usize,
                    height as usize,
                    x,
                    y,
                    text,
                    color,
                    13,
                    5,
                )
            },
        );

        // Bottom row: never draw note-name labels there.
        let y_mid = top_end as i32 + 2;
        crate::render_shared::draw_note_name_labels(
            &mid_prio,
            &mid_pc,
            &mid_color,
            1,
            transpose_pc,
            4,
            y_mid,
            |x, y, text, color| {
                crate::pixel_font::draw_text_u32(
                    &mut buffer,
                    width as usize,
                    height as usize,
                    x,
                    y,
                    text,
                    color,
                    13,
                    5,
                )
            },
        );
    }

    // Settings overlay.
    let (gear, panel, rows) = settings_layout(width, height);
    // Gear button
    fill_rect(
        &mut buffer,
        width as usize,
        height as usize,
        gear,
        0x00222222,
    );
    crate::pixel_font::draw_text_u32(
        &mut buffer,
        width as usize,
        height as usize,
        gear.x + 4,
        gear.y + 4,
        "SET",
        0x00FFFFFF,
        13,
        5,
    );

    if show_settings {
        fill_rect(
            &mut buffer,
            width as usize,
            height as usize,
            panel,
            0x00111111,
        );
        stroke_rect(
            &mut buffer,
            width as usize,
            height as usize,
            panel,
            0x00333333,
        );

        // Row 1: TAP
        draw_checkbox_row(
            &mut buffer,
            width as usize,
            height as usize,
            rows[0],
            settings.play_on_tap,
            "TAP",
        );
        draw_checkbox_row(
            &mut buffer,
            width as usize,
            height as usize,
            rows[1],
            settings.show_note_names,
            "LBL",
        );
        draw_checkbox_row(
            &mut buffer,
            width as usize,
            height as usize,
            rows[2],
            settings.show_roman_chords,
            "ROM",
        );

        let backend_label = match settings.audio_backend {
            UiAudioBackend::Synth => "SYN",
            _ => "MID",
        };
        draw_value_row(
            &mut buffer,
            width as usize,
            height as usize,
            rows[3],
            "AUD",
            backend_label,
        );

        // Tuning slider.
        draw_slider_row(
            &mut buffer,
            width as usize,
            height as usize,
            rows[4],
            "A4",
            settings.a4_tuning_hz,
            430,
            450,
        );
    }

    buffer.present().unwrap();
}

fn fill_rect(buf: &mut [u32], w: usize, h: usize, r: RectI32, color: u32) {
    let x0 = r.x.max(0) as usize;
    let y0 = r.y.max(0) as usize;
    let x1 = (r.x + r.w).min(w as i32).max(0) as usize;
    let y1 = (r.y + r.h).min(h as i32).max(0) as usize;

    for y in y0..y1 {
        let row = y * w;
        for x in x0..x1 {
            buf[row + x] = color;
        }
    }
}

fn stroke_rect(buf: &mut [u32], w: usize, h: usize, r: RectI32, color: u32) {
    fill_rect(
        buf,
        w,
        h,
        RectI32 {
            x: r.x,
            y: r.y,
            w: r.w,
            h: 1,
        },
        color,
    );
    fill_rect(
        buf,
        w,
        h,
        RectI32 {
            x: r.x,
            y: r.y + r.h - 1,
            w: r.w,
            h: 1,
        },
        color,
    );
    fill_rect(
        buf,
        w,
        h,
        RectI32 {
            x: r.x,
            y: r.y,
            w: 1,
            h: r.h,
        },
        color,
    );
    fill_rect(
        buf,
        w,
        h,
        RectI32 {
            x: r.x + r.w - 1,
            y: r.y,
            w: 1,
            h: r.h,
        },
        color,
    );
}

fn draw_checkbox_row(buf: &mut [u32], w: usize, h: usize, row: RectI32, value: bool, label: &str) {
    let box_r = RectI32 {
        x: row.x + 6,
        y: row.y + 5,
        w: 10,
        h: 10,
    };
    fill_rect(buf, w, h, box_r, 0x00000000);
    stroke_rect(buf, w, h, box_r, 0x00777777);
    if value {
        fill_rect(
            buf,
            w,
            h,
            RectI32 {
                x: box_r.x + 2,
                y: box_r.y + 2,
                w: box_r.w - 4,
                h: box_r.h - 4,
            },
            0x00FFFFFF,
        );
    }

    crate::pixel_font::draw_text_u32(buf, w, h, row.x + 22, row.y + 3, label, 0x00FFFFFF, 13, 5);
}

fn draw_value_row(buf: &mut [u32], w: usize, h: usize, row: RectI32, label: &str, value: &str) {
    crate::pixel_font::draw_text_u32(buf, w, h, row.x + 6, row.y + 3, label, 0x00FFFFFF, 13, 5);
    crate::pixel_font::draw_text_u32(buf, w, h, row.x + 64, row.y + 3, value, 0x00FFFFFF, 13, 5);
}

fn draw_slider_row(
    buf: &mut [u32],
    w: usize,
    h: usize,
    row: RectI32,
    label: &str,
    value: u16,
    min: u16,
    max: u16,
) {
    crate::pixel_font::draw_text_u32(buf, w, h, row.x + 6, row.y + 3, label, 0x00FFFFFF, 13, 5);

    let value_str = value.to_string();
    crate::pixel_font::draw_text_u32(
        buf,
        w,
        h,
        row.x + 28,
        row.y + 3,
        &value_str,
        0x00FFFFFF,
        13,
        5,
    );

    let bar = RectI32 {
        x: row.x + 60,
        y: row.y + 7,
        w: row.w - 66,
        h: 6,
    };
    fill_rect(buf, w, h, bar, 0x00111111);
    stroke_rect(buf, w, h, bar, 0x00333333);

    let t = ((value.saturating_sub(min)) as f32 / (max - min) as f32).clamp(0.0, 1.0);
    let fill_w = (t * (bar.w as f32)).round() as i32;
    if fill_w > 0 {
        fill_rect(
            buf,
            w,
            h,
            RectI32 {
                x: bar.x,
                y: bar.y,
                w: fill_w,
                h: bar.h,
            },
            0x00AAAAAA,
        );
    }
}

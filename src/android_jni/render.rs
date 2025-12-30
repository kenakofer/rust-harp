use crate::android_frontend::AndroidFrontend;
use crate::layout;

use jni::objects::JIntArray;
use jni::sys::{jint, jlong};
use jni::JNIEnv;

pub(crate) fn render_strings(env: JNIEnv, handle: jlong, width: jint, height: jint, out_pixels: JIntArray) {
    let w = width.max(0) as usize;
    let h = height.max(0) as usize;
    if w == 0 || h == 0 {
        return;
    }

    let (top_chord, middle_chord, show_note_names, transpose_pc, visuals) = if handle != 0 {
        let frontend = unsafe { &*(handle as *const AndroidFrontend) };
        let eng = frontend.engine();
        (
            eng.active_chord_for_row(crate::rows::RowId::Top),
            eng.active_chord_for_row(crate::rows::RowId::Middle)
                .unwrap_or_else(|| crate::chord::Chord::new_triad(crate::notes::UnkeyedNote(0))),
            frontend.show_note_names(),
            eng.transpose().wrap_to_octave(),
            frontend.note_visuals_snapshot(),
        )
    } else {
        (
            None,
            crate::chord::Chord::new_triad(crate::notes::UnkeyedNote(0)),
            false,
            0,
            Vec::new(),
        )
    };

    fn draw_text(pixels: &mut [i32], w: usize, h: usize, x_left: i32, y_top: i32, text: &str, color: i32) {
        // +30% over the old 2x scale => 2.6x.
        crate::pixel_font::draw_text_i32(pixels, w, h, x_left, y_top, text, color, 13, 5)
    }

    let len = w * h;
    let mut pixels = vec![0xFF000000u32 as i32; len];

    let mut positions_storage: Vec<f32> = Vec::new();
    let mut cache_guard: Option<std::sync::MutexGuard<'_, crate::layout::NotePositionsCache>> = None;
    let positions: &[f32] = if handle != 0 {
        let frontend = unsafe { &*(handle as *const AndroidFrontend) };
        cache_guard = Some(frontend.layout_cache.lock().unwrap());
        cache_guard.as_mut().unwrap().android(w as f32)
    } else {
        positions_storage = layout::compute_note_positions_android(w as f32);
        &positions_storage
    };

    let (top_end, mid_end) = crate::render_shared::row_band_bounds(h);

    let style = crate::render_best::BestStyle {
        root_prio: 3,
        root_color: 0xFFFF0000u32 as i32,
        chord_prio: 2,
        chord_color: 0xFFFFFFFFu32 as i32,
        inactive_prio: 1,
        inactive_color: 0xFF333333u32 as i32,
    };

    let (top_prio, top_color, top_pc) = crate::render_best::compute_best_per_x(
        w,
        positions,
        top_chord,
        transpose_pc,
        style,
        true,
    );
    let (mid_prio, mid_color, mid_pc) = crate::render_best::compute_best_per_x(
        w,
        positions,
        Some(middle_chord),
        transpose_pc,
        style,
        true,
    );
    let (bot_prio, bot_color, bot_pc) = crate::render_best::compute_best_per_x(
        w,
        positions,
        Some(middle_chord.invert()),
        transpose_pc,
        style,
        true,
    );

    crate::render_shared::fill_string_bands(
        &mut pixels,
        w,
        h,
        top_end,
        mid_end,
        &top_prio,
        &top_color,
        &mid_prio,
        &mid_color,
        &bot_prio,
        &bot_color,
    );

    // Note-on visuals: strike = flash+fade; strum = widen then shrink.
    if !visuals.is_empty() {
        use crate::android_frontend::{NoteVisualKind, NOTE_STRIKE_VIS_MS, NOTE_STRUM_VIS_MS};
        use crate::rows::RowId;

        const INACTIVE_GRAY: i32 = 0xFF333333u32 as i32;

        fn blend_to_white(c: i32, f: f32) -> i32 {
            let f = f.clamp(0.0, 1.0);
            let cu = c as u32;
            let r = ((cu >> 16) & 0xFF) as f32;
            let g = ((cu >> 8) & 0xFF) as f32;
            let b = (cu & 0xFF) as f32;
            let nr = (r + (255.0 - r) * f).round() as u32;
            let ng = (g + (255.0 - g) * f).round() as u32;
            let nb = (b + (255.0 - b) * f).round() as u32;
            (0xFF00_0000u32 | (nr << 16) | (ng << 8) | nb) as i32
        }

        fn blend_towards(src: i32, dst: i32, f: f32) -> i32 {
            let f = f.clamp(0.0, 1.0);
            let su = src as u32;
            let du = dst as u32;
            let sr = ((su >> 16) & 0xFF) as f32;
            let sg = ((su >> 8) & 0xFF) as f32;
            let sb = (su & 0xFF) as f32;
            let dr = ((du >> 16) & 0xFF) as f32;
            let dg = ((du >> 8) & 0xFF) as f32;
            let db = (du & 0xFF) as f32;
            let r = (sr + (dr - sr) * f).round() as u32;
            let g = (sg + (dg - sg) * f).round() as u32;
            let b = (sb + (db - sb) * f).round() as u32;
            (0xFF00_0000u32 | (r << 16) | (g << 8) | b) as i32
        }

        let now = std::time::Instant::now();
        for e in visuals {
            // Note positions are indexed by absolute UnkeyedNote (including ANDROID_LOWEST_NOTE offset).
            // Using wrap_to_octave() would point into the "dummy" (-inf) region and hide visuals.
            let ni_i16 = e.note.as_i16();
            if ni_i16 < 0 {
                continue;
            }
            let ni = ni_i16 as usize;
            if ni >= positions.len() {
                continue;
            }
            let x = positions[ni];
            if !x.is_finite() {
                continue;
            }
            let xi = x.round() as i32;
            if xi < 0 || xi >= w as i32 {
                continue;
            }
            let (y0, y1) = match e.row {
                RowId::Top => (0usize, top_end),
                RowId::Middle => (top_end, mid_end),
                RowId::Bottom => (mid_end, h),
            };

            // Match the string's existing color; skip inactive (dim gray) strings.
            let base_color = match e.row {
                RowId::Top => top_color[xi as usize],
                RowId::Middle => mid_color[xi as usize],
                RowId::Bottom => bot_color[xi as usize],
            };
            if base_color == INACTIVE_GRAY {
                continue;
            }
            let highlight_color = blend_to_white(base_color, 0.75);

            let age_ms = now.saturating_duration_since(e.at).as_millis() as f32;

            let (dur_ms, width_px, mix) = match e.kind {
                NoteVisualKind::Strike => {
                    let t = (age_ms / NOTE_STRIKE_VIS_MS as f32).clamp(0.0, 1.0);
                    (NOTE_STRIKE_VIS_MS as f32, 14i32, (1.0 - t).powf(1.6))
                }
                NoteVisualKind::Strum => {
                    let t = (age_ms / NOTE_STRUM_VIS_MS as f32).clamp(0.0, 1.0);
                    let w0 = 12.0;
                    let w1 = 1.0;
                    let wcur = (w1 + (w0 - w1) * (1.0 - t)).round() as i32;
                    (NOTE_STRUM_VIS_MS as f32, wcur.max(1), 0.9)
                }
            };
            let _ = dur_ms; // (kept for readability/debugging)

            let half = width_px / 2;
            let x0 = (xi - half).max(0) as usize;
            let x1 = (xi + half).min((w - 1) as i32) as usize;
            for x in x0..=x1 {
                for y in y0..y1 {
                    let idx = y * w + x;
                    pixels[idx] = blend_towards(pixels[idx], highlight_color, mix);
                }
            }
        }
    }

    if show_note_names {
        crate::render_shared::draw_note_name_labels(
            &top_prio,
            &top_pc,
            &top_color,
            2,
            transpose_pc,
            4,
            2,
            |x, y, text, color| draw_text(&mut pixels, w, h, x, y, text, color),
        );

        // Middle row labels.
        let y_mid = top_end as i32 + 2;
        crate::render_shared::draw_note_name_labels(
            &mid_prio,
            &mid_pc,
            &mid_color,
            2,
            transpose_pc,
            4,
            y_mid,
            |x, y, text, color| draw_text(&mut pixels, w, h, x, y, text, color),
        );
    }

    let _ = bot_pc; // (kept for future labels)
    let _ = env.set_int_array_region(out_pixels, 0, &pixels);
}

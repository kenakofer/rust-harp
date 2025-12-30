use crate::notes;

pub(crate) fn row_band_bounds(height: usize) -> (usize, usize) {
    // 40% top, 40% middle, 20% bottom
    let top_end = height * 2 / 5;
    let mid_end = height * 4 / 5;
    (top_end, mid_end)
}

pub(crate) fn fill_string_bands<P: Copy>(
    pixels: &mut [P],
    width: usize,
    height: usize,
    top_end: usize,
    mid_end: usize,
    top_prio: &[u8],
    top_color: &[P],
    mid_prio: &[u8],
    mid_color: &[P],
    bot_prio: &[u8],
    bot_color: &[P],
) {
    debug_assert_eq!(pixels.len(), width * height);
    debug_assert_eq!(top_prio.len(), width);
    debug_assert_eq!(top_color.len(), width);
    debug_assert_eq!(mid_prio.len(), width);
    debug_assert_eq!(mid_color.len(), width);
    debug_assert_eq!(bot_prio.len(), width);
    debug_assert_eq!(bot_color.len(), width);

    for xi in 0..width {
        if top_prio[xi] != 0 {
            let color = top_color[xi];
            for y in 0..top_end {
                pixels[y * width + xi] = color;
            }
        }
        if mid_prio[xi] != 0 {
            let color = mid_color[xi];
            for y in top_end..mid_end {
                pixels[y * width + xi] = color;
            }
        }
        if bot_prio[xi] != 0 {
            let color = bot_color[xi];
            for y in mid_end..height {
                pixels[y * width + xi] = color;
            }
        }
    }
}

pub(crate) fn draw_note_name_labels<C: Copy>(
    prio: &[u8],
    pc: &[u8],
    color: &[C],
    min_prio: u8,
    transpose_pc: i16,
    x_pad: i32,
    y_top: i32,
    mut draw: impl FnMut(i32, i32, &str, C),
) {
    debug_assert_eq!(prio.len(), pc.len());
    debug_assert_eq!(prio.len(), color.len());

    for (xi, pr) in prio.iter().enumerate() {
        if *pr < min_prio {
            continue;
        }
        let pc = pc[xi];
        if pc == 255 {
            continue;
        }
        let label = notes::pitch_class_label(pc as i16, transpose_pc);
        draw(xi as i32 + x_pad, y_top, label, color[xi]);
    }
}

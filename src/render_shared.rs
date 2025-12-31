use crate::notes;

pub(crate) fn fill_string_bands_rows<P: Copy>(
    pixels: &mut [P],
    width: usize,
    height: usize,
    y_edges: &[usize],
    rows: &[(&[u8], &[P])],
) {
    debug_assert_eq!(pixels.len(), width * height);
    debug_assert_eq!(y_edges.first().copied().unwrap_or(0), 0);
    debug_assert_eq!(y_edges.last().copied().unwrap_or(0), height);
    debug_assert_eq!(y_edges.len(), rows.len() + 1);

    for &(prio, color) in rows {
        debug_assert_eq!(prio.len(), width);
        debug_assert_eq!(color.len(), width);
    }

    for xi in 0..width {
        for r in 0..rows.len() {
            let (prio, color) = rows[r];
            if prio[xi] == 0 {
                continue;
            }
            let c = color[xi];
            for y in y_edges[r]..y_edges[r + 1] {
                pixels[y * width + xi] = c;
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

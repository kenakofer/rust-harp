/// Minimal 5x7 pixel font and text blitter used by both Android and desktop UIs.
///
/// This is intentionally tiny: we only include glyphs we currently need.

pub fn glyph_5x7(ch: char) -> [u8; 7] {
    match ch {
        // Notes
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        'C' => [
            0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
        ],
        'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'G' => [
            0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110,
        ],
        '#' => [
            0b01010, 0b11111, 0b01010, 0b01010, 0b11111, 0b01010, 0b01010,
        ],
        'b' => [
            0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b10001, 0b11110,
        ],

        // Desktop settings panel labels (uppercase)
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],

        // Separators
        ' ' => [0; 7],
        _ => [0; 7],
    }
}

pub fn draw_text_u32(
    pixels: &mut [u32],
    w: usize,
    h: usize,
    x_left: i32,
    y_top: i32,
    text: &str,
    color: u32,
    scale_num: i32,
    scale_den: i32,
) {
    let map = |u: i32| (u * scale_num) / scale_den;

    let char_w: i32 = map(5);
    let spacing: i32 = map(1).max(1);

    let mut x = x_left;
    for ch in text.chars() {
        let g = glyph_5x7(ch);
        for (row, bits) in g.iter().enumerate() {
            for col in 0..5 {
                if (bits & (1 << (4 - col))) == 0 {
                    continue;
                }

                let x0 = x + map(col as i32);
                let x1 = x + map(col as i32 + 1);
                let y0 = y_top + map(row as i32);
                let y1 = y_top + map(row as i32 + 1);

                for py in y0..y1 {
                    for px in x0..x1 {
                        if px < 0 || py < 0 {
                            continue;
                        }
                        let (px, py) = (px as usize, py as usize);
                        if px >= w || py >= h {
                            continue;
                        }
                        pixels[py * w + px] = color;
                    }
                }
            }
        }
        x += char_w + spacing;
    }
}

pub fn draw_text_i32(
    pixels: &mut [i32],
    w: usize,
    h: usize,
    x_left: i32,
    y_top: i32,
    text: &str,
    color: i32,
    scale_num: i32,
    scale_den: i32,
) {
    let map = |u: i32| (u * scale_num) / scale_den;

    let char_w: i32 = map(5);
    let spacing: i32 = map(1).max(1);

    let mut x = x_left;
    for ch in text.chars() {
        let g = glyph_5x7(ch);
        for (row, bits) in g.iter().enumerate() {
            for col in 0..5 {
                if (bits & (1 << (4 - col))) == 0 {
                    continue;
                }

                let x0 = x + map(col as i32);
                let x1 = x + map(col as i32 + 1);
                let y0 = y_top + map(row as i32);
                let y1 = y_top + map(row as i32 + 1);

                for py in y0..y1 {
                    for px in x0..x1 {
                        if px < 0 || py < 0 {
                            continue;
                        }
                        let (px, py) = (px as usize, py as usize);
                        if px >= w || py >= h {
                            continue;
                        }
                        pixels[py * w + px] = color;
                    }
                }
            }
        }
        x += char_w + spacing;
    }
}

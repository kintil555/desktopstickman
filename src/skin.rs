//! Model pemain kustom: sprite sheet PNG (grid cols x rows), dipilih per state (idle/jalan/lari/lompat).
use crate::{
    config::{Config, Skin},
    stick::{Canvas, Stick, V, WALK},
};
use std::{fs::File, io::BufReader};

pub struct Sheet {
    w: usize,
    h: usize,
    /// RGBA8
    px: Vec<u8>,
}

pub fn load(path: &str) -> Option<Sheet> {
    let mut dec = png::Decoder::new(BufReader::new(File::open(path).ok()?));
    dec.set_transformations(png::Transformations::normalize_to_color8());
    let mut rd = dec.read_info().ok()?;
    let mut buf = vec![0; rd.output_buffer_size()];
    let info = rd.next_frame(&mut buf).ok()?;
    let src = &buf[..info.buffer_size()];
    let px: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => src.to_vec(),
        png::ColorType::Rgb => (0..src.len() / 3)
            .flat_map(|i| [src[i * 3], src[i * 3 + 1], src[i * 3 + 2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => (0..src.len() / 2)
            .flat_map(|i| [src[i * 2], src[i * 2], src[i * 2], src[i * 2 + 1]])
            .collect(),
        png::ColorType::Grayscale => src.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return None,
    };
    Some(Sheet {
        w: info.width as usize,
        h: info.height as usize,
        px,
    })
}

/// Gambar frame sprite sesuai state `s`; `o` = titik pijak (tengah-bawah frame).
/// `hang` = (sudut ayun, titik tangan): sprite digantung dari tangan dan diputar.
pub fn draw_player(
    c: &mut Canvas,
    sh: &Sheet,
    cfg: &Config,
    s: &Stick,
    o: V,
    seated: bool,
    hang: Option<(f32, V)>,
) {
    let k = &cfg.skin;
    let sp = s.vx.abs();
    let a = if seated || hang.is_some() {
        &k.idle
    } else if s.oy > 0.0 {
        &k.jump
    } else if sp > WALK * 0.7 {
        &k.run
    } else if sp > 8.0 {
        &k.walk
    } else {
        &k.idle
    };
    let (cols, rows) = (k.cols.max(1) as usize, k.rows.max(1) as usize);
    let (fw, fh) = (sh.w / cols, sh.h / rows);
    if fw == 0 || fh == 0 {
        return;
    }
    let col = (s.t * a.fps as f32) as usize % (a.frames.clamp(1, cols as u32) as usize);
    let (x0, y0) = (col * fw, (a.row as usize).min(rows - 1) * fh);
    let ps = k.scale * if seated { k.seat_scale } else { 1.0 };
    let f = s.face;
    let mirror = if k.faces_left { f > 0.0 } else { f < 0.0 };
    let (fwf, fhf) = (fw as f32, fh as f32);
    // bergantung: bagian atas sprite di tangan; selain itu telapak kaki di `o`
    let base = match hang {
        Some((_, pv)) => V {
            x: pv.x,
            y: pv.y + fhf * ps,
        },
        None => o,
    };
    let (ax, ay) = (
        base.x + f * k.off_x as f32 * ps,
        base.y + k.off_y as f32 * ps,
    );
    let cell = c.cell;
    let (gx0, gx1, gy0, gy1) = if hang.is_some() {
        (0, c.w, 0, c.h)
    } else {
        (
            ((ax - fwf * ps / 2.0) / cell).floor().max(0.0) as usize,
            (((ax + fwf * ps / 2.0) / cell).ceil().max(0.0) as usize).min(c.w),
            ((ay - fhf * ps) / cell).floor().max(0.0) as usize,
            ((ay / cell).ceil().max(0.0) as usize).min(c.h),
        )
    };
    let (sn, cs) = hang.map_or((0.0, 1.0), |(t, _)| t.sin_cos());
    for gy in gy0..gy1 {
        for gx in gx0..gx1 {
            let (mut wx, mut wy) = ((gx as f32 + 0.5) * cell, (gy as f32 + 0.5) * cell);
            if let Some((_, pv)) = hang {
                // balik-putar sekitar tangan agar sprite berayun
                let (dx, dy) = (wx - pv.x, wy - pv.y);
                (wx, wy) = (pv.x + dx * cs - dy * sn, pv.y + dx * sn + dy * cs);
            }
            let ly = (wy - ay) / ps + fhf;
            if !(0.0..fhf).contains(&ly) {
                continue;
            }
            let lx = (wx - ax) / ps + fwf / 2.0;
            if !(0.0..fwf).contains(&lx) {
                continue;
            }
            let ix = if mirror {
                fw - 1 - lx as usize
            } else {
                lx as usize
            };
            let q = ((y0 + ly as usize) * sh.w + x0 + ix) * 4;
            let p = &sh.px[q..q + 4];
            if p[3] < 128 {
                continue;
            }
            let ch = |i: usize| {
                if k.tint {
                    p[i] as u32 * cfg.color[i] as u32 / 255
                } else {
                    p[i] as u32
                }
            };
            c.px[gy * c.w + gx] = 0xFF00_0000 | ch(0) << 16 | ch(1) << 8 | ch(2);
        }
    }
}

/// Font angka 3x5 (bit 2 = kolom kiri) untuk nomor baris di pratinjau grid.
const DIGITS: [[u8; 5]; 10] = [
    [7, 5, 5, 5, 7],
    [2, 6, 2, 2, 7],
    [7, 1, 7, 4, 7],
    [7, 1, 7, 1, 7],
    [5, 5, 7, 1, 1],
    [7, 4, 7, 1, 7],
    [7, 4, 7, 5, 7],
    [7, 1, 1, 1, 1],
    [7, 5, 7, 5, 7],
    [7, 5, 7, 1, 7],
];

fn put(out: &mut [u8], w: usize, x: usize, y: usize, c: [u8; 3]) {
    let o = (y * w + x) * 4;
    if x < w && o + 3 < out.len() {
        out[o..o + 3].copy_from_slice(&c);
    }
}

/// Pratinjau sprite sheet + garis grid (kolom x baris). Frame yang dipakai tiap animasi
/// diwarnai (idle hijau, jalan biru, lari merah, lompat kuning) dan nomor baris tampil di kiri.
/// Hasil RGBA8 `(lebar, tinggi, piksel)` yang muat di `max_w` x `max_h`.
pub fn grid_preview(k: &Skin, max_w: usize, max_h: usize) -> Option<(usize, usize, Vec<u8>)> {
    let sh = load(&k.path)?;
    let (cols, rows) = (k.cols.max(1) as usize, k.rows.max(1) as usize);
    let (fw, fh) = ((sh.w / cols).max(1), (sh.h / rows).max(1));
    let fit = (max_w as f32 / sh.w as f32).min(max_h as f32 / sh.h as f32);
    let sc = if fit >= 1.0 { fit.floor() } else { fit };
    let px = |v: usize| (v as f32 * sc) as usize;
    let (w, h) = (px(sh.w).max(1), px(sh.h).max(1));
    let mut out = vec![0u8; w * h * 4];
    // sheet di atas kotak-kotak abu-abu (supaya area transparan terlihat)
    for y in 0..h {
        for x in 0..w {
            let bg: u32 = if (x / 8 + y / 8) % 2 == 0 { 70 } else { 90 };
            let sx = ((x as f32 / sc) as usize).min(sh.w - 1);
            let sy = ((y as f32 / sc) as usize).min(sh.h - 1);
            let p = &sh.px[(sy * sh.w + sx) * 4..][..4];
            let a = p[3] as u32;
            let o = (y * w + x) * 4;
            for (i, &c) in p[..3].iter().enumerate() {
                out[o + i] = ((c as u32 * a + bg * (255 - a)) / 255) as u8;
            }
            out[o + 3] = 255;
        }
    }
    // warnai frame yang dipakai tiap animasi
    let tints = [
        (&k.idle, [80u32, 200, 90]),
        (&k.walk, [70, 140, 255]),
        (&k.run, [255, 80, 80]),
        (&k.jump, [255, 220, 60]),
    ];
    for (a, col) in tints {
        let r = (a.row as usize).min(rows - 1);
        let n = a.frames.clamp(1, cols as u32) as usize;
        let (x1, y0, y1) = (px(n * fw), px(r * fh), px((r + 1) * fh));
        for y in y0..y1.min(h) {
            for x in 0..x1.min(w) {
                let o = (y * w + x) * 4;
                for (i, c) in col.iter().enumerate() {
                    out[o + i] = ((out[o + i] as u32 * 65 + c * 35) / 100) as u8;
                }
            }
        }
    }
    // garis grid
    let line = [255, 138, 42];
    for c in 0..=cols {
        let x = px(c * fw).min(w - 1);
        for y in 0..h {
            put(&mut out, w, x, y, line);
        }
    }
    for r in 0..=rows {
        let y = px(r * fh).min(h - 1);
        for x in 0..w {
            put(&mut out, w, x, y, line);
        }
    }
    // nomor baris di sel kolom pertama
    for r in 0..rows {
        let s = r.to_string();
        if px(fh) < 16 || px(fw) < s.len() * 8 + 6 {
            continue;
        }
        let (ox, oy) = (3, px(r * fh) + 3);
        for y in 0..11 {
            for x in 0..s.len() * 8 {
                put(&mut out, w, ox - 1 + x, oy - 1 + y, [0, 0, 0]);
            }
        }
        for (di, ch) in s.bytes().enumerate() {
            for (ry, &bits) in DIGITS[(ch - b'0') as usize].iter().enumerate() {
                for rx in 0..3 {
                    if (bits >> (2 - rx)) & 1 == 1 {
                        for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                            let (x, y) = (ox + di * 8 + rx * 2 + dx, oy + ry * 2 + dy);
                            put(&mut out, w, x, y, [255, 255, 255]);
                        }
                    }
                }
            }
        }
    }
    Some((w, h, out))
}

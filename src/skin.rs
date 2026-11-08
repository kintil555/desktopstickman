//! Model pemain kustom: sprite sheet PNG (grid cols x rows), dipilih per state (idle/jalan/lari/lompat).
use crate::{
    config::Config,
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

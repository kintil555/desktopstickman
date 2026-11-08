//! Drop shadow pixel: siluet kanvas digeser per sel (+ pelebaran opsional), alpha datar, digambar di bawah sosok.
use crate::{config::Config, stick::Canvas};

/// Pelebaran siluet `r` sel (kotak): tepi tetap keras per sel, bukan gradasi halus.
fn dilate(m: &mut [bool], w: usize, h: usize, r: usize) {
    let src = m.to_vec();
    for y in 0..h {
        let (y0, y1) = (y.saturating_sub(r), (y + r).min(h - 1));
        for x in 0..w {
            let (x0, x1) = (x.saturating_sub(r), (x + r).min(w - 1));
            m[y * w + x] = (y0..=y1).any(|j| (x0..=x1).any(|i| src[j * w + i]));
        }
    }
}

pub fn apply(c: &mut Canvas, cfg: &Config) {
    if !cfg.shadow || cfg.shadow_opacity <= 0.0 {
        return;
    }
    let (w, h) = (c.w as i32, c.h as i32);
    let dx = (cfg.shadow_dx as f32 / c.cell).round() as i32;
    let dy = (cfg.shadow_dy as f32 / c.cell).round() as i32;
    let mut m = vec![false; c.w * c.h];
    for y in 0..h {
        for x in 0..w {
            let a = c.px[(y * w + x) as usize] >> 24;
            let (sx, sy) = (x + dx, y + dy);
            if a != 0 && (0..w).contains(&sx) && (0..h).contains(&sy) {
                m[(sy * w + sx) as usize] = true;
            }
        }
    }
    if cfg.shadow_blur > 0 {
        dilate(&mut m, c.w, c.h, cfg.shadow_blur as usize);
    }
    // alpha datar (tanpa gradasi), premultiplied ARGB (format layered window)
    let (col, a) = (cfg.shadow_color.map(|v| v as f32), cfg.shadow_opacity);
    let ch = |i: usize| (col[i] * a).round() as u32;
    let px = ((a * 255.0).round() as u32) << 24 | ch(0) << 16 | ch(1) << 8 | ch(2);
    for (p, on) in c.px.iter_mut().zip(m) {
        if on && *p >> 24 == 0 {
            *p = px;
        }
    }
}

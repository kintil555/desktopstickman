//! Drop shadow: bayangan dari siluet kanvas (geser + blur kotak per sel), digambar di bawah sosok.
use crate::{config::Config, stick::Canvas};

fn blur(m: &mut [f32], w: usize, h: usize, r: usize) {
    let n = (2 * r + 1) as f32;
    let mut tmp = vec![0.0f32; w * h];
    for y in 0..h {
        for x in 0..w {
            let (a, b) = (x.saturating_sub(r), (x + r).min(w - 1));
            tmp[y * w + x] = (a..=b).map(|i| m[y * w + i]).sum::<f32>() / n;
        }
    }
    for y in 0..h {
        let (a, b) = (y.saturating_sub(r), (y + r).min(h - 1));
        for x in 0..w {
            m[y * w + x] = (a..=b).map(|j| tmp[j * w + x]).sum::<f32>() / n;
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
    let mut m = vec![0.0f32; c.w * c.h];
    for y in 0..h {
        for x in 0..w {
            let a = c.px[(y * w + x) as usize] >> 24;
            let (sx, sy) = (x + dx, y + dy);
            if a != 0 && (0..w).contains(&sx) && (0..h).contains(&sy) {
                m[(sy * w + sx) as usize] = a as f32 / 255.0;
            }
        }
    }
    if cfg.shadow_blur > 0 {
        blur(&mut m, c.w, c.h, cfg.shadow_blur as usize);
    }
    let col = cfg.shadow_color.map(|v| v as f32);
    for (p, a) in c.px.iter_mut().zip(m) {
        let a = a * cfg.shadow_opacity;
        if *p >> 24 == 0 && a > 0.004 {
            // premultiplied ARGB (format layered window)
            let ch = |i: usize| (col[i] * a).round() as u32;
            *p = ((a * 255.0).round() as u32) << 24 | ch(0) << 16 | ch(1) << 8 | ch(2);
        }
    }
}

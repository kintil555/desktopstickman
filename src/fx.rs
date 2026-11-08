//! Debu sederhana: puff lari + semburan saat mendarat (palet dari HTML: DUST_C).
use crate::stick::{Canvas, Rng};

const TONE: [u32; 3] = [0xFFFF_FFFF, 0xFFF1_F1F4, 0xFFDA_DADF];

pub struct Dust {
    pub x: f32,
    h: f32,
    vx: f32,
    vy: f32,
    life: f32,
    max: f32,
    r: f32,
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Puff kecil di belakang kaki; `back` = arah belakang (-arah lari), `k` 0..1.
pub fn run_puff(l: &mut Vec<Dust>, x: f32, back: f32, k: f32, rng: &mut Rng) {
    let life = lerp(0.26, 0.34, rng.f());
    l.push(Dust {
        x: x + back * 2.0,
        h: 0.0,
        vx: back * lerp(12.0, 28.0, k),
        vy: 0.0,
        life,
        max: life,
        r: lerp(2.0, 5.0, k) * (0.92 + rng.f() * 0.16),
    });
    cap(l);
}

/// Semburan dua sisi saat mendarat/lepas landas; `k` 0..1 = kekuatan.
pub fn burst(l: &mut Vec<Dust>, x: f32, k: f32, rng: &mut Rng) {
    for dir in [-1.0f32, 1.0] {
        for i in 1..=(1 + (k > 0.3) as i32 + (k > 0.65) as i32) {
            let f = 1.0 - i as f32 * 0.32;
            let life = lerp(0.18, 0.68, k) * (1.0 - i as f32 * 0.12);
            l.push(Dust {
                x: x + dir * (3.0 + i as f32 * 2.0),
                h: 0.0,
                vx: dir * lerp(30.0, 300.0, k) * f * (0.9 + rng.f() * 0.2),
                vy: 0.0,
                life,
                max: life,
                r: lerp(3.0, 9.0, k) * f,
            });
        }
        for _ in 0..(1.0 + k * 4.0) as i32 {
            let life = (0.14 + 0.28 * k) * (0.7 + rng.f() * 0.6);
            l.push(Dust {
                x: x + dir * rng.f() * (14.0 + 76.0 * k),
                h: 0.0,
                vx: dir * (8.0 + rng.f() * (10.0 + 30.0 * k)),
                vy: 3.0 + rng.f() * 8.0,
                life,
                max: life,
                r: 1.0 + rng.f() * 2.0,
            });
        }
    }
    cap(l);
}

fn cap(l: &mut Vec<Dust>) {
    if l.len() > 140 {
        l.drain(..l.len() - 140);
    }
}

pub fn update(l: &mut Vec<Dust>, dt: f32) {
    for d in l.iter_mut() {
        d.x += d.vx * dt;
        d.h += d.vy * dt;
        d.vx *= 1.0 - (3.0 * dt).min(1.0);
        d.life -= dt;
    }
    l.retain(|d| d.life > 0.0);
}

/// `ox`: x kanvas untuk posisi dunia `wx`; `floor_y`: garis lantai di kanvas.
pub fn draw(c: &mut Canvas, d: &Dust, wx: f32, ox: f32, floor_y: f32) {
    let t = 1.0 - d.life / d.max;
    let th = if t < 0.45 {
        1.0
    } else {
        (1.0 - (t - 0.45) / 0.55).max(0.0)
    };
    let e = (t / 0.3).min(1.0);
    let ease = 1.0 - (1.0 - e) * (1.0 - e) * (1.0 - e);
    let r = d.r * (0.3 + 0.7 * ease) * th;
    if r < 0.6 {
        return;
    }
    let (cx, cy) = (d.x - wx + ox, floor_y - d.h - r * 0.6);
    let cell = c.cell;
    let x0 = ((cx - r) / cell).floor().max(0.0) as usize;
    let x1 = (((cx + r) / cell).ceil().max(0.0) as usize).min(c.w.saturating_sub(1));
    let y0 = ((cy - r) / cell).floor().max(0.0) as usize;
    let y1 = (((cy + r) / cell).ceil().max(0.0) as usize).min(c.h.saturating_sub(1));
    for gy in y0..=y1 {
        let py = (gy as f32 + 0.5) * cell;
        if py > floor_y {
            continue;
        }
        for gx in x0..=x1 {
            let (dx, dy) = ((gx as f32 + 0.5) * cell - cx, py - cy);
            if dx.hypot(dy) > r {
                continue;
            }
            let l = (-dx * 0.55 - dy * 0.7) / r.max(1.0);
            c.px[gy * c.w + gx] = if l > 0.15 {
                TONE[0]
            } else if l > -0.3 {
                TONE[1]
            } else {
                TONE[2]
            };
        }
    }
}

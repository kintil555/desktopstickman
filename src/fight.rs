//! Pertarungan antar stickman: HP, kilat putih, ledakan partikel putih saat mati, respawn.
//! Pola efek mengikuti resep VFX prosedural: fase waktu (kilat -> ledakan -> hilang), partikel
//! berekor searah kecepatan, cincin gelombang kejut (easeOut), memudar dengan Bayer (tanpa alpha).
use crate::stick::{Canvas, Rng};

/// Lama tubuh memutih sebelum meledak.
pub const FLASH_T: f32 = 0.3;
/// Detik sejak mati sampai hidup lagi.
pub const RESPAWN_T: f32 = 3.0;
const RING_T: f32 = 0.35;

const B4: [[u8; 4]; 4] = [
    [0, 8, 2, 10],
    [12, 4, 14, 6],
    [3, 11, 1, 9],
    [15, 7, 13, 5],
];

fn bayer(x: usize, y: usize) -> f32 {
    (B4[y & 3][x & 3] as f32 + 0.5) / 16.0
}

/// Partikel ledakan (koordinat jendela, px layar).
pub struct Part {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    life: f32,
    max: f32,
}

pub struct Fighter {
    pub hp: i32,
    pub alive: bool,
    /// Detik sejak mati.
    pub t_dead: f32,
    /// Jeda antar pukulan, pukulan yang belum mengenai.
    pub cd: f32,
    pub pending: bool,
    /// Sisa kilat putih saat kena pukul / saat lahir kembali.
    pub hit: f32,
    pub born: f32,
    pub burst: bool,
    pub hidden: bool,
    pub parts: Vec<Part>,
}

impl Fighter {
    pub fn new(hp: i32) -> Self {
        Self {
            hp,
            alive: true,
            t_dead: 0.0,
            cd: 0.5,
            pending: false,
            hit: 0.0,
            born: 0.0,
            burst: false,
            hidden: false,
            parts: Vec::new(),
        }
    }

    /// Seberapa putih tubuh saat hidup (0..1).
    pub fn white(&self) -> f32 {
        let hit = (self.hit / 0.1).clamp(0.0, 1.0) * 0.7;
        hit.max((self.born / FLASH_T).clamp(0.0, 1.0))
    }

    pub fn die(&mut self) {
        self.alive = false;
        self.t_dead = 0.0;
        self.pending = false;
    }

    /// Letupan partikel putih dari titik `(cx, cy)` (koordinat jendela).
    pub fn explode(&mut self, rng: &mut Rng, cx: f32, cy: f32) {
        self.burst = true;
        for _ in 0..46 {
            let a = rng.f() * std::f32::consts::TAU;
            let sp = 60.0 + rng.f() * 170.0;
            let max = 0.35 + rng.f() * 0.35;
            self.parts.push(Part {
                x: cx,
                y: cy,
                vx: a.cos() * sp,
                vy: a.sin() * sp - 30.0,
                life: max,
                max,
            });
        }
    }

    pub fn update(&mut self, dt: f32) {
        for p in &mut self.parts {
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            p.vy += 160.0 * dt;
            p.vx *= 1.0 - 1.5 * dt;
            p.life -= dt;
        }
        self.parts.retain(|p| p.life > 0.0);
    }

    /// Ledakan selesai: tidak ada partikel dan cincin sudah hilang.
    pub fn burst_done(&self) -> bool {
        self.burst && self.parts.is_empty() && self.t_dead - FLASH_T >= RING_T
    }
}

/// Putihkan piksel tubuh: `k` 0..1 dengan Bayer (tanpa blending, tetap pixel art).
pub fn whiten(c: &mut Canvas, k: f32) {
    if k <= 0.0 {
        return;
    }
    for y in 0..c.h {
        for x in 0..c.w {
            let p = &mut c.px[y * c.w + x];
            if (*p >> 24) != 0 && bayer(x, y) < k {
                *p = 0xFFFF_FFFF;
            }
        }
    }
}

/// Satu piksel kanvas di posisi layar `(x, y)`; `fade` < 0.35 memudar dengan Bayer.
fn put(c: &mut Canvas, x: f32, y: f32, col: u32, fade: f32) {
    if x < 0.0 || y < 0.0 {
        return;
    }
    let (ix, iy) = ((x / c.cell) as usize, (y / c.cell) as usize);
    if ix >= c.w || iy >= c.h || (fade < 0.35 && bayer(ix, iy) > fade / 0.35) {
        return;
    }
    c.px[iy * c.w + ix] = col;
}

/// Gambar ledakan: cincin gelombang kejut + partikel putih berekor.
pub fn draw_burst(c: &mut Canvas, f: &Fighter, cx: f32, cy: f32) {
    let cell = c.cell;
    let age = f.t_dead - FLASH_T;
    if age < RING_T {
        let k = (age / RING_T).clamp(0.0, 1.0);
        let r = 10.0 + 66.0 * (1.0 - (1.0 - k).powi(3));
        let th = 1.0 + 6.0 * (1.0 - k);
        let rad = r + th;
        let x0 = ((cx - rad) / cell).floor().max(0.0) as usize;
        let y0 = ((cy - rad) / cell).floor().max(0.0) as usize;
        let x1 = (((cx + rad) / cell).ceil().max(0.0) as usize).min(c.w);
        let y1 = (((cy + rad) / cell).ceil().max(0.0) as usize).min(c.h);
        for iy in y0..y1 {
            for ix in x0..x1 {
                let dx = (ix as f32 + 0.5) * cell - cx;
                let dy = (iy as f32 + 0.5) * cell - cy;
                if (dx.hypot(dy) - r).abs() <= th / 2.0 && bayer(ix, iy) <= 1.0 - k {
                    c.px[iy * c.w + ix] = 0xFFFF_FFFF;
                }
            }
        }
    }
    for p in &f.parts {
        let sp = p.vx.hypot(p.vy).max(1.0);
        let (ux, uy) = (p.vx / sp, p.vy / sp);
        let len = (1.0 + sp * 0.035).min(8.0) as i32;
        let fade = p.life / p.max;
        for s in 0..=len {
            let d = s as f32 * cell;
            let col = if s == 0 { 0xFFFF_FFFF } else { 0xFFC8_C8C8 };
            put(c, p.x - ux * d, p.y - uy * d, col, fade);
        }
    }
}

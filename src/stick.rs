//! Gait prosedural + rasterizer sel (port dari `Stickman Pixel.html`: pose1/ik/drawFig).
use crate::config::Config;
use std::f32::consts::{PI, TAU};

pub const S: f32 = 1.25 * 1.02;
const R: f32 = S / 2.4;
const HEADR: f32 = 7.5 * S;
const NECK: f32 = 2.0 * S;
const TORSO: f32 = 18.0 * S;
const UA: f32 = 12.0 * S;
const FA: f32 = 12.0 * S;
const TH: f32 = 14.0 * S;
const SH: f32 = 14.0 * S;
const LEG: f32 = 28.0 * S;
const A: f32 = LEG * 0.56;
const LIFT: f32 = LEG * 0.26;
const WALK: f32 = 350.0;
const ACC: f32 = 2600.0 * WALK / 460.0;
const GRAV: f32 = 2400.0;
const JUMP_V: f32 = 520.0;

#[derive(Clone, Copy, Default)]
pub struct V {
    pub x: f32,
    pub y: f32,
}

fn v(x: f32, y: f32) -> V {
    V { x, y }
}

fn ik(a: V, b: V, l1: f32, l2: f32, bend: f32) -> V {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let d = dx.hypot(dy).clamp((l1 - l2).abs() + 0.01, l1 + l2 - 0.01);
    let an = ((l1 * l1 + d * d - l2 * l2) / (2.0 * l1 * d))
        .clamp(-1.0, 1.0)
        .acos();
    let t = dy.atan2(dx) + bend * an;
    v(a.x + l1 * t.cos(), a.y + l1 * t.sin())
}

pub struct Rng(u32);

impl Rng {
    pub fn new(seed: u32) -> Self {
        Self(seed | 1)
    }
    pub fn f(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 >> 8) as f32 / (1u32 << 24) as f32
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Idle,
    Walk,
    Run,
}

pub struct Stick {
    /// Posisi x (px layar, relatif tepi kiri monitor).
    pub x: f32,
    pub vx: f32,
    /// Tinggi di atas lantai (>=0) dan kecepatan naik.
    pub oy: f32,
    vup: f32,
    pub face: f32,
    phase: f32,
    t: f32,
    mode: Mode,
    timer: f32,
    dir: f32,
    jump_t: f32,
}

pub struct Pose {
    hip: V,
    sh: V,
    head: V,
    legs: [(V, V); 2],
    arms: [(V, V); 2],
}

impl Stick {
    pub fn new(x: f32) -> Self {
        Self {
            x,
            vx: 0.0,
            oy: 0.0,
            vup: 0.0,
            face: 1.0,
            phase: 0.0,
            t: 0.0,
            mode: Mode::Idle,
            timer: 1.0,
            dir: 1.0,
            jump_t: 3.0,
        }
    }

    fn on(&self) -> bool {
        self.oy <= 0.0
    }

    pub fn update(&mut self, dt: f32, cfg: &Config, rng: &mut Rng, max_x: f32) {
        let margin = 40.0;
        self.t += dt;
        self.timer -= dt;
        if self.timer <= 0.0 {
            let r = rng.f();
            self.mode = if r < 0.35 {
                Mode::Idle
            } else if r < 0.8 || !cfg.run {
                Mode::Walk
            } else {
                Mode::Run
            };
            self.timer = 1.5 + rng.f() * 3.5;
            self.dir = if rng.f() < 0.5 { -1.0 } else { 1.0 };
        }
        // balik arah di tepi layar
        if self.x < margin {
            self.dir = 1.0;
        } else if self.x > max_x - margin {
            self.dir = -1.0;
        }
        let target = match self.mode {
            Mode::Idle => 0.0,
            Mode::Walk => WALK * 0.4,
            Mode::Run => WALK,
        } * cfg.speed
            * self.dir;
        let a = ACC * dt * if self.on() { 1.0 } else { 0.4 };
        self.vx += (target - self.vx).clamp(-a, a);
        if self.vx.abs() > 5.0 {
            self.face = self.vx.signum();
        }
        self.x = (self.x + self.vx * dt).clamp(0.0, max_x);

        self.jump_t -= dt;
        if self.jump_t <= 0.0 {
            if cfg.jump && self.on() && rng.f() < 0.6 {
                self.vup = JUMP_V;
                self.oy = 0.01;
            }
            self.jump_t = 2.0 + rng.f() * 4.0;
        }
        if !self.on() {
            self.oy += self.vup * dt;
            self.vup -= GRAV * dt;
            if self.oy <= 0.0 {
                self.oy = 0.0;
                self.vup = 0.0;
            }
        }
        // satu siklus penuh = 4A jarak tempuh (tanpa foot-sliding)
        if self.on() {
            self.phase = (self.phase + self.vx.abs() * dt / (4.0 * A)) % 1.0;
        }
    }

    pub fn pose(&self) -> Pose {
        let (f, sp, on) = (self.face, self.vx.abs(), self.on());
        let m = if on { (sp / 60.0).clamp(0.0, 1.0) } else { 0.0 };
        let dm = if self.vx >= 0.0 { 1.0 } else { -1.0 };
        let ph = self.phase;
        let br = (self.t * 2.4).sin();
        let idle = if on { 1.0 - m } else { 0.0 };
        let sk = if on { (sp / WALK).clamp(0.0, 1.0) } else { 0.0 };
        let mut hip = v(
            0.0,
            -(LEG * (0.94 - 0.14 * sk)
                - R * (m * 4.0 * (TAU * ph).cos().abs() + idle * (3.0 + 3.0 * br))),
        );
        let ae = A * if self.vx * f < 0.0 { 0.72 } else { 1.0 };
        let mut ft = [V::default(); 2];
        for (i, foot) in ft.iter_mut().enumerate() {
            *foot = if on {
                let u = (ph + i as f32 * 0.5) % 1.0;
                let (wx, wy) = if u < 0.5 {
                    (ae * (1.0 - 4.0 * u), 0.0)
                } else {
                    let s = (u - 0.5) * 2.0;
                    let e = s * s * (3.0 - 2.0 * s);
                    (ae * (2.0 * e - 1.0), LIFT * (PI * s).sin())
                };
                let rest = R * if i == 0 { f * 9.0 } else { -f * 9.0 };
                v(rest + (dm * wx - rest) * m, -wy * m)
            } else {
                let t = (self.vup / 900.0).clamp(0.0, 1.0);
                if i == 1 {
                    v(R * -f * 8.0, R * -(4.0 + 10.0 * t))
                } else {
                    v(R * f * 12.0, R * -(10.0 + 18.0 * t))
                }
            };
        }
        if on {
            let l = LEG * 0.985;
            for q in &ft {
                let (dx, dy) = (q.x - hip.x, q.y - hip.y);
                if dy > 0.0 && dx * dx + dy * dy > l * l {
                    hip.y = q.y - (l * l - dx * dx).max(0.0).sqrt();
                }
            }
        }
        let legs = [0, 1].map(|i| (ik(hip, ft[i], TH, SH, -f), ft[i]));
        let lean = (self.vx / WALK).clamp(-1.0, 1.0) * 0.12;
        let sh = v(hip.x + lean.sin() * TORSO, hip.y - lean.cos() * TORSO);
        let ha = lean * 1.3;
        let head = v(
            sh.x + ha.sin() * (NECK + HEADR),
            sh.y - ha.cos() * (NECK + HEADR),
        );
        let ay = R
            * (46.0 - 8.0 * m * (TAU * ph).sin().abs() - if on { 0.0 } else { 14.0 }
                + idle * br * 1.5);
        let sw = R * dm * 20.0 * m * (TAU * ph).cos();
        let h1 = v(sh.x + sw - R * (1.0 - m) * f * 4.0, sh.y + ay);
        let h0 = v(sh.x - sw + R * (1.0 - m) * f * 4.0, sh.y + ay);
        let arms = [h0, h1].map(|h| (ik(sh, h, UA, FA, f), h));
        Pose {
            hip,
            sh,
            head,
            legs,
            arms,
        }
    }
}

pub struct Canvas {
    pub w: usize,
    pub h: usize,
    cell: f32,
    pub px: Vec<u32>,
}

fn dist_seg(p: V, a: V, b: V) -> f32 {
    let (abx, aby) = (b.x - a.x, b.y - a.y);
    let l2 = abx * abx + aby * aby;
    let t = if l2 < 1e-6 {
        0.0
    } else {
        (((p.x - a.x) * abx + (p.y - a.y) * aby) / l2).clamp(0.0, 1.0)
    };
    (p.x - a.x - abx * t).hypot(p.y - a.y - aby * t)
}

fn pack(c: [u8; 3]) -> u32 {
    0xFF00_0000 | (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32
}

impl Canvas {
    pub fn new(w_px: u32, h_px: u32, cell: u32) -> Self {
        let (w, h) = ((w_px / cell) as usize, (h_px / cell) as usize);
        Self {
            w,
            h,
            cell: cell as f32,
            px: vec![0; w * h],
        }
    }

    fn fill(&mut self, lo: V, hi: V, col: u32, hit: impl Fn(V) -> bool) {
        let c = self.cell;
        let x0 = (lo.x / c).floor().max(0.0) as usize;
        let y0 = (lo.y / c).floor().max(0.0) as usize;
        let x1 = ((hi.x / c).ceil().max(0.0) as usize).min(self.w.saturating_sub(1));
        let y1 = ((hi.y / c).ceil().max(0.0) as usize).min(self.h.saturating_sub(1));
        for cy in y0..=y1 {
            for cx in x0..=x1 {
                if hit(v((cx as f32 + 0.5) * c, (cy as f32 + 0.5) * c)) {
                    self.px[cy * self.w + cx] = col;
                }
            }
        }
    }

    fn limb(&mut self, pts: &[V], col: u32) {
        for w in pts.windows(2) {
            let (a, b) = (w[0], w[1]);
            let lo = v(a.x.min(b.x) - 2.0, a.y.min(b.y) - 2.0);
            let hi = v(a.x.max(b.x) + 2.0, a.y.max(b.y) + 2.0);
            self.fill(lo, hi, col, |p| dist_seg(p, a, b) <= 2.0);
        }
    }

    fn ring(&mut self, c: V, r: f32, col: u32) {
        let lo = v(c.x - r - 2.0, c.y - r - 2.0);
        let hi = v(c.x + r + 2.0, c.y + r + 2.0);
        self.fill(lo, hi, col, |p| {
            ((p.x - c.x).hypot(p.y - c.y) - r).abs() <= 2.0
        });
    }

    /// Bayangan elips di lantai (mengecil saat melompat).
    pub fn shadow(&mut self, c: V, k: f32) {
        let (rx, ry) = (14.0 * k, 3.0 * k);
        let lo = v(c.x - rx, c.y - ry);
        let hi = v(c.x + rx, c.y + ry);
        self.fill(lo, hi, 0x4000_0000, |p| {
            let (dx, dy) = ((p.x - c.x) / rx, (p.y - c.y) / ry);
            dx * dx + dy * dy <= 1.0
        });
    }

    /// Gambar sosok; `o` = titik pijak lantai (px kanvas).
    pub fn draw(&mut self, q: &Pose, o: V, cfg: &Config) {
        let (cf, cb) = (pack(cfg.color_f), pack(cfg.color_b));
        let t = |p: V| v(o.x + p.x, o.y + p.y);
        let (hip, sh) = (t(q.hip), t(q.sh));
        let lim = |s: &(V, V)| [hip, t(s.0), t(s.1)];
        self.limb(&lim(&q.legs[1]), cb);
        self.limb(&[sh, t(q.arms[1].0), t(q.arms[1].1)], cb);
        self.limb(&[hip, sh], cf);
        self.limb(&lim(&q.legs[0]), cf);
        self.ring(t(q.head), HEADR, cf);
        self.limb(&[sh, t(q.arms[0].0), t(q.arms[0].1)], cf);
    }
}

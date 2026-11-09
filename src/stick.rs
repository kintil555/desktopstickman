//! Gait prosedural + rasterizer sel (port dari `Stickman Pixel.html`: pose1/ik/drawFig).
use crate::{config::Config, fx, veh};
use std::f32::consts::{PI, TAU};

pub const S: f32 = 1.25 * 1.02;
const R: f32 = S / 2.4;
pub(crate) const HEADR: f32 = 7.5 * S;
pub(crate) const NECK: f32 = 2.0 * S;
pub(crate) const TORSO: f32 = 18.0 * S;
pub(crate) const UA: f32 = 12.0 * S;
pub(crate) const FA: f32 = 12.0 * S;
pub(crate) const TH: f32 = 14.0 * S;
pub(crate) const SH: f32 = 14.0 * S;
pub(crate) const LEG: f32 = 28.0 * S;
const A: f32 = LEG * 0.56;
const LIFT: f32 = LEG * 0.26;
pub const WALK: f32 = 350.0;
const ACC: f32 = 2600.0 * WALK / 460.0;
pub(crate) const GRAV: f32 = 2400.0;
const JUMP_V: f32 = 760.0;
/// Durasi animasi mendarat (jongkok lalu bangkit).
const LAND_T: f32 = 0.4;
/// Lama satu ayunan serangan (detik).
pub const ATK_T: f32 = 0.32;

fn ease(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

#[derive(Clone, Copy, Default)]
pub struct V {
    pub x: f32,
    pub y: f32,
}

fn v(x: f32, y: f32) -> V {
    V { x, y }
}

pub(crate) fn ik(a: V, b: V, l1: f32, l2: f32, bend: f32) -> V {
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
    pub t: f32,
    mode: Mode,
    timer: f32,
    dir: f32,
    jump_t: f32,
    /// Ketinggian terbang di atas lantai (jendela ikut naik) + dorongan jet 0..1.
    pub alt: f32,
    pub fly: Fly,
    pub thr: f32,
    alt_to: f32,
    fly_t: f32,
    fly_cd: f32,
    puff_t: f32,
    peak: f32,
    pub fx: Vec<fx::Dust>,
    /// Asap api kendaraan dan percikan masuk/keluar kendaraan.
    pub smoke: Vec<fx::Smoke>,
    pub sparks: Vec<fx::Spark>,
    summon_t: f32,
    smoke_acc: f32,
    /// Lama jatuh (detik) dan seberapa terangkat tangan saat mulai jatuh (1 = dari gantungan).
    fall_t: f32,
    arm0: f32,
    /// Sisa waktu animasi mendarat + kedalaman jongkok 0..1.
    land_t: f32,
    land_k: f32,
    /// Sasaran serang: `(x lawan, jarak berhenti)`; None = gerak acak.
    pub goal: Option<(f32, f32)>,
    atk_t: f32,
    atk_arm: usize,
}

pub struct Pose {
    pub hip: V,
    pub sh: V,
    pub head: V,
    pub legs: [(V, V); 2],
    pub arms: [(V, V); 2],
}

#[derive(Clone, Copy, PartialEq)]
pub enum Fly {
    No,
    Up,
    Cruise,
    Down,
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
            alt: 0.0,
            fly: Fly::No,
            thr: 0.0,
            alt_to: 0.0,
            fly_t: 0.0,
            fly_cd: 8.0,
            puff_t: 0.0,
            peak: 0.0,
            fx: Vec::new(),
            smoke: Vec::new(),
            sparks: Vec::new(),
            summon_t: 0.0,
            smoke_acc: 0.0,
            fall_t: 0.0,
            arm0: 0.0,
            land_t: 0.0,
            land_k: 0.0,
            goal: None,
            atk_t: 0.0,
            atk_arm: 0,
        }
    }

    /// Mulai ayunan serangan dengan lengan `arm` (0 = depan, 1 = belakang).
    pub fn attack(&mut self, arm: usize) {
        self.atk_t = ATK_T;
        self.atk_arm = arm.min(1);
    }

    pub fn attacking(&self) -> bool {
        self.atk_t > 0.0
    }

    /// Ayunan sudah sampai titik pukul (pertengahan akhir).
    pub fn hit_ready(&self) -> bool {
        self.atk_t > 0.0 && self.atk_t <= ATK_T * 0.45
    }

    /// Terpental oleh pukulan: dorongan mendatar + lompatan kecil.
    pub fn knock(&mut self, vx: f32) {
        self.vx = vx;
        if self.fly == Fly::No && self.on() {
            self.vup = 320.0;
            self.oy = 0.01;
            self.peak = 0.0;
            self.fall_t = 0.0;
            self.arm0 = 0.0;
        }
    }

    /// Putaran tambahan item saat mengayun (radian, searah jarum jam untuk hadap kanan).
    pub fn swing(&self) -> f32 {
        if self.atk_t <= 0.0 {
            return 0.0;
        }
        (-0.6 + 2.8 * ease(1.0 - self.atk_t / ATK_T)) * self.face
    }

    /// Mendarat setelah dilepas dari gantungan mouse (`x` relatif monitor, `h` = tinggi jatuh).
    pub fn land(&mut self, x: f32, vx: f32, h: f32, rng: &mut Rng) {
        self.x = x;
        self.vx = vx;
        self.oy = 0.0;
        self.vup = 0.0;
        self.mode = Mode::Idle;
        self.timer = 1.0;
        self.land_t = LAND_T;
        self.land_k = (h / 200.0).clamp(0.4, 1.0);
        self.fall_t = 0.0;
        fx::spawn_land(&mut self.fx, x, fx::dust_k(h), rng);
    }

    /// Mulai jatuh dari gantungan: tangan sudah terangkat, lalu perlahan turun ke pose jatuh.
    pub fn begin_fall(&mut self) {
        // debu lari lama dibuang: posisinya relatif ke s.x lama, bukan ke stickman yang jatuh
        self.fx.clear();
        self.fall_t = 0.0;
        self.arm0 = 1.0;
    }

    /// Satu frame jatuh bebas (posisi diatur `Hang`): pose udara.
    pub fn fall_tick(&mut self, vx: f32, dt: f32) {
        // jendela mengikuti telapak kaki; debu hanya muncul saat mendarat (`land`)
        self.fx.clear();
        self.oy = 1.0;
        self.vx = vx;
        self.t += dt;
        self.fall_t += dt;
        if vx.abs() > 5.0 {
            self.face = vx.signum();
        }
    }

    pub fn on(&self) -> bool {
        self.oy <= 0.0
    }

    pub fn update(&mut self, dt: f32, cfg: &Config, rng: &mut Rng, max_x: f32, max_alt: f32) {
        let margin = 40.0;
        self.t += dt;
        fx::update(&mut self.fx, dt);
        fx::update_air(&mut self.smoke, &mut self.sparks, dt);
        self.land_t = (self.land_t - dt).max(0.0);
        self.atk_t = (self.atk_t - dt).max(0.0);
        if self.x < margin {
            self.dir = 1.0;
        } else if self.x > max_x - margin {
            self.dir = -1.0;
        }
        if self.fly != Fly::No {
            self.update_fly(dt, cfg, rng, max_x);
            return;
        }
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
        // sasaran serang: kejar lawan, berhenti di jangkauan
        if let Some((g, stop)) = self.goal {
            let dx = g - self.x;
            self.dir = if dx < 0.0 { -1.0 } else { 1.0 };
            self.mode = if dx.abs() <= stop {
                Mode::Idle
            } else if dx.abs() > 220.0 && cfg.run {
                Mode::Run
            } else {
                Mode::Walk
            };
        }
        let target = match self.mode {
            Mode::Idle => 0.0,
            Mode::Walk => WALK * 0.4,
            Mode::Run => WALK,
        } * cfg.speed
            * self.dir;
        let target = if self.land_t > 0.0 { 0.0 } else { target };
        let a = ACC * dt * if self.on() { 1.0 } else { 0.4 };
        self.vx += (target - self.vx).clamp(-a, a);
        if self.vx.abs() > 5.0 {
            self.face = self.vx.signum();
        }
        if let Some((g, _)) = self.goal {
            self.face = if g < self.x { -1.0 } else { 1.0 };
        }
        self.x = (self.x + self.vx * dt).clamp(0.0, max_x);

        self.fly_cd -= dt;
        if cfg.fly && self.goal.is_none() && self.fly_cd <= 0.0 && self.on() {
            self.fly = Fly::Up;
            self.alt_to = 100.0 + rng.f() * (max_alt - 100.0).max(0.0);
            self.thr = 1.0;
            self.summon_t = 0.18;
            fx::spawn_land(&mut self.fx, self.x, 0.6, rng);
            return;
        }
        self.jump_t -= dt;
        if self.jump_t <= 0.0 {
            if cfg.jump && self.on() && rng.f() < 0.6 {
                self.vup = JUMP_V;
                self.oy = 0.01;
                self.peak = 0.0;
                self.fall_t = 0.0;
                self.arm0 = 0.0;
            }
            self.jump_t = 2.0 + rng.f() * 4.0;
        }
        if !self.on() {
            self.oy += self.vup * dt;
            self.vup -= GRAV * dt;
            self.peak = self.peak.max(self.oy);
            if self.vup < 0.0 {
                self.fall_t += dt;
            }
            if self.oy <= 0.0 {
                if self.peak >= 24.0 {
                    self.land_t = LAND_T;
                    self.land_k = (self.peak / 200.0).clamp(0.35, 1.0);
                }
                self.fall_t = 0.0;
                if cfg.dust && -self.vup > 260.0 && self.peak >= 24.0 {
                    fx::spawn_land(&mut self.fx, self.x, fx::dust_k(self.peak), rng);
                }
                self.oy = 0.0;
                self.vup = 0.0;
            }
        }
        // satu siklus penuh = 4A jarak tempuh (tanpa foot-sliding)
        if self.on() {
            self.phase = (self.phase + self.vx.abs() * dt / (4.0 * A)) % 1.0;
            self.puff_t -= dt;
            if cfg.dust && self.vx.abs() > 180.0 && self.puff_t <= 0.0 {
                self.puff_t = 0.05;
                fx::spawn_run(&mut self.fx, self.x, self.vx.signum(), self.vx.abs(), rng);
            }
        }
    }

    fn update_fly(&mut self, dt: f32, cfg: &Config, rng: &mut Rng, max_x: f32) {
        if self.summon_t > 0.0 {
            self.summon_t = (self.summon_t - dt).max(0.0);
            if cfg.dust {
                fx::spawn_summon(&mut self.sparks, self.x, self.alt + 30.0, dt, rng);
            }
        }
        let tgt = if self.fly == Fly::Cruise { 260.0 } else { 0.0 } * cfg.speed * self.dir;
        let a = ACC * 0.5 * dt;
        self.vx += (tgt - self.vx).clamp(-a, a);
        if self.vx.abs() > 5.0 {
            self.face = self.vx.signum();
        }
        self.x = (self.x + self.vx * dt).clamp(0.0, max_x);
        match self.fly {
            Fly::Up => {
                self.alt += 380.0 * dt;
                self.thr = 1.0;
                if self.alt < 30.0 && cfg.dust {
                    self.puff_t -= dt;
                    if self.puff_t <= 0.0 {
                        self.puff_t = 0.08;
                        fx::spawn_land(&mut self.fx, self.x, 0.25, rng);
                    }
                }
                if self.alt >= self.alt_to {
                    self.alt = self.alt_to;
                    self.fly = Fly::Cruise;
                    self.fly_t = 4.0 + rng.f() * 5.0;
                    self.dir = if rng.f() < 0.5 { -1.0 } else { 1.0 };
                }
            }
            Fly::Cruise => {
                self.thr = 0.5;
                self.fly_t -= dt;
                if self.fly_t <= 0.0 {
                    self.fly = Fly::Down;
                }
            }
            _ => {
                self.thr = 0.35;
                self.alt -= 300.0 * dt;
                if self.alt <= 0.0 {
                    self.alt = 0.0;
                    self.fly = Fly::No;
                    self.thr = 0.0;
                    self.fly_cd = 12.0 + rng.f() * 20.0;
                    self.timer = 1.0;
                    self.mode = Mode::Idle;
                    if cfg.dust {
                        fx::spawn_land(&mut self.fx, self.x, 0.7, rng);
                        fx::spawn_burst(&mut self.sparks, self.x, 30.0, rng);
                    }
                }
            }
        }
        if cfg.dust {
            self.emit_smoke(dt, rng);
        }
    }

    /// Asap tipis dari ujung api tiap nozzle (kecepatan sebanding dorongan jet).
    fn emit_smoke(&mut self, dt: f32, rng: &mut Rng) {
        if self.fly == Fly::No {
            return;
        }
        self.smoke_acc += dt * (8.0 + 12.0 * self.thr);
        let l = (5.0 + 9.0 * self.thr) * 0.9 * veh::PC;
        while self.smoke_acc >= 1.0 {
            self.smoke_acc -= 1.0;
            for j in 0..2 {
                let n = veh::nozzle(j, self.face);
                let x = self.x + n.x + (rng.f() - 0.5) * 3.0 * veh::PC;
                let h = (self.alt - n.y - l).max(1.0);
                fx::spawn_smoke(&mut self.smoke, x, h, rng);
            }
        }
    }

    /// Lompat sekarang (tombol pintas).
    pub fn jump_now(&mut self) {
        if self.fly == Fly::No && self.on() {
            self.vup = JUMP_V;
            self.oy = 0.01;
            self.peak = 0.0;
            self.fall_t = 0.0;
            self.arm0 = 0.0;
        }
    }

    /// Pratinjau panel: berlari di tempat 2,5 detik, diam 1,5 detik, berulang.
    pub fn demo(&mut self, dt: f32, cfg: &Config) {
        self.t += dt;
        self.vx = if self.t % 4.0 < 2.5 {
            WALK * cfg.speed
        } else {
            0.0
        };
        self.face = 1.0;
        self.phase = (self.phase + self.vx * dt / (4.0 * A)) % 1.0;
    }

    /// Mulai terbang segera (tes dump / tombol pintas).
    pub fn force_fly(&mut self) {
        self.fly_cd = 0.0;
    }

    /// Offset vertikal goyangan kendaraan saat melayang (px).
    pub fn bob(&self) -> f32 {
        if self.fly == Fly::Cruise {
            ((self.t * 1000.0 / 240.0).sin() * 0.8).round() * 2.0
        } else {
            0.0
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
        let u = 1.0 - self.land_t / LAND_T;
        let landing = on && self.land_t > 0.0;
        // jongkok saat mendarat: turun cepat, bangkit pelan
        let crouch = if u < 0.25 {
            ease(u / 0.25)
        } else {
            1.0 - ease((u - 0.25) / 0.75)
        };
        if landing {
            hip.y += self.land_k * LEG * 0.3 * crouch;
        }
        let tuck = if on { 0.0 } else { ease(self.fall_t / 0.35) };
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
                    v(R * -f * 8.0, R * -(4.0 + 10.0 * t) - tuck * LEG * 0.3)
                } else {
                    v(R * f * 12.0, R * -(10.0 + 18.0 * t) - tuck * LEG * 0.3)
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
        // tangan terangkat: perlahan saat jatuh (kecuali sudah naik dari gantungan), turun saat bangkit
        let au = if !on {
            self.arm0 + (0.85 - self.arm0) * ease(self.fall_t / 0.6)
        } else if landing {
            0.8 * (1.0 - ease(u))
        } else {
            0.0
        };
        let up = |side: f32, h: V| {
            let l = (UA + FA) * 0.96;
            let r = v(sh.x + side * 2.5f32.sin() * l, sh.y + 2.5f32.cos() * l);
            v(h.x + (r.x - h.x) * au, h.y + (r.y - h.y) * au)
        };
        let (h0, h1) = (up(-1.0, h0), up(1.0, h1));
        let mut hands = [h0, h1];
        if self.atk_t > 0.0 {
            // ayunan: dari belakang-atas, lewat atas kepala, ke depan-bawah
            let a = -0.6 + 2.8 * ease(1.0 - self.atk_t / ATK_T);
            let r = (UA + FA) * 0.92;
            hands[self.atk_arm] = v(sh.x + f * a.sin() * r, sh.y - a.cos() * r);
        }
        let arms = hands.map(|h| (ik(sh, h, UA, FA, f), h));
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
    pub cell: f32,
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

    fn limb(&mut self, pts: &[V], col: u32, tw: f32) {
        for s in pts.windows(2) {
            let (a, b) = (s[0], s[1]);
            let lo = v(a.x.min(b.x) - tw, a.y.min(b.y) - tw);
            let hi = v(a.x.max(b.x) + tw, a.y.max(b.y) + tw);
            self.fill(lo, hi, col, |p| dist_seg(p, a, b) <= tw);
        }
    }

    fn ring(&mut self, c: V, r: f32, col: u32, tw: f32) {
        let lo = v(c.x - r - tw, c.y - r - tw);
        let hi = v(c.x + r + tw, c.y + r + tw);
        self.fill(lo, hi, col, |p| {
            ((p.x - c.x).hypot(p.y - c.y) - r).abs() <= tw
        });
    }

    fn disc(&mut self, c: V, r: f32, col: u32) {
        let lo = v(c.x - r, c.y - r);
        let hi = v(c.x + r, c.y + r);
        self.fill(lo, hi, col, |p| (p.x - c.x).hypot(p.y - c.y) <= r);
    }

    /// Gambar sosok; `o` = titik pijak lantai (px kanvas). Shading + outline ala `pixelize` di HTML.
    pub fn draw(&mut self, q: &Pose, o: V, cfg: &Config) {
        const CF: u32 = 1;
        const CB: u32 = 2;
        let mut fig = Canvas {
            w: self.w,
            h: self.h,
            cell: self.cell,
            px: vec![0; self.w * self.h],
        };
        let t = |p: V| v(o.x + p.x, o.y + p.y);
        let (hip, sh) = (t(q.hip), t(q.sh));
        let lim = |s: &(V, V)| [hip, t(s.0), t(s.1)];
        let bd = &cfg.body;
        fig.limb(&lim(&q.legs[1]), CB, bd.limb_w);
        fig.limb(&[sh, t(q.arms[1].0), t(q.arms[1].1)], CB, bd.limb_w);
        fig.limb(&[hip, sh], CF, bd.torso_w);
        fig.limb(&lim(&q.legs[0]), CF, bd.limb_w);
        let hr = HEADR * bd.head_size;
        // kepala kecil digeser ke bahu supaya tetap menempel (tidak mengambang)
        let hd = t(q.head);
        let (dx, dy) = (sh.x - hd.x, sh.y - hd.y);
        let k = (HEADR - hr) / dx.hypot(dy).max(0.001);
        let hc = v(hd.x + dx * k, hd.y + dy * k);
        if bd.head_fill {
            fig.disc(hc, hr + bd.head_w, CF);
        } else {
            fig.ring(hc, hr, CF, bd.head_w);
        }
        fig.limb(&[sh, t(q.arms[0].0), t(q.arms[0].1)], CF, bd.limb_w);

        // turunan warna: selisih sama seperti palet HTML (HI/FS/BS terhadap F/B)
        let sh_col = |c: [u8; 3], d: [i16; 3]| -> u32 {
            pack([0, 1, 2].map(|i| {
                (c[i] as i16 + (d[i] as f32 * bd.shade_k).round() as i16).clamp(0, 255) as u8
            }))
        };
        let back = cfg.back();
        let (f, b) = (pack(cfg.color), pack(back));
        let hi = sh_col(cfg.color, [17, 56, 62]);
        let fs = sh_col(cfg.color, [-24, -22, -14]);
        let bs = sh_col(back, [-44, -22, -8]);
        let (hi, fs, bs) = if bd.shade { (hi, fs, bs) } else { (f, f, b) };
        let ol = pack(bd.outline_color);
        let (w, h) = (self.w, self.h);
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                let c = fig.px[i];
                if c != 0 {
                    let up = y == 0 || fig.px[i - w] == 0;
                    let dn = y == h - 1 || fig.px[i + w] == 0;
                    self.px[i] = if c == CF {
                        if up {
                            hi
                        } else if dn {
                            fs
                        } else {
                            f
                        }
                    } else if dn {
                        bs
                    } else {
                        b
                    };
                } else if bd.outline
                    && ((x > 0 && fig.px[i - 1] != 0)
                        || (x + 1 < w && fig.px[i + 1] != 0)
                        || (y > 0 && fig.px[i - w] != 0)
                        || (y + 1 < h && fig.px[i + w] != 0))
                {
                    self.px[i] = ol;
                }
            }
        }
    }
}

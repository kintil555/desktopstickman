//! Bergantung di kursor mouse: panjat prosedural (hand-over-hand) -> bergantung sebagai pendulum.
//! Kursor bergerak ke kanan = badan berayun ke kiri; ayunan dan miring bergantung kecepatan kursor.
use crate::stick::{ik, Pose, FA, GRAV, HEADR, LEG, NECK, SH, TH, TORSO, UA, V};
use std::f32::consts::TAU;

/// Posisi tangan (kursor) di dalam jendela overlay (px dari atas).
pub const HAND_Y: f32 = 60.0;
const ARM: f32 = (UA + FA) * 0.92;
const CLIMB_T: f32 = 1.1;
const CYCLES: f32 = 2.5;
const REACH_IN: f32 = 40.0;
/// Panjang pendulum efektif (px) ke pusat massa.
const PEND: f32 = 60.0;

#[derive(Clone, Copy, PartialEq)]
pub enum Phase {
    Climb,
    Hang,
    Fall,
}

pub struct Hang {
    pub phase: Phase,
    /// Posisi tangan = kursor (px layar absolut).
    pub hand: V,
    pub face: f32,
    th: f32,
    w: f32,
    p: f32,
    delta: V,
    vxs: f32,
    /// Waktu sejak menggantung (mencegah klik yang sama langsung melepas).
    pub age: f32,
    /// Fase jatuh: posisi telapak kaki, kecepatan.
    pub feet: V,
    pub vel: V,
}

fn v(x: f32, y: f32) -> V {
    V { x, y }
}

fn ease(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

impl Hang {
    /// `feet` = telapak kaki stickman di layar saat ini; `cursor` = posisi kursor.
    pub fn grab(cursor: V, feet: V, face: f32) -> Self {
        let shoulder = v(feet.x, feet.y - (LEG * 0.94 + TORSO));
        let start = v(cursor.x, cursor.y + ARM + REACH_IN);
        Self {
            phase: Phase::Climb,
            hand: cursor,
            face,
            th: 0.0,
            w: 0.0,
            p: 0.0,
            // selisih posisi awal vs rumus panjat; meluruh di awal agar tidak melompat
            delta: v(shoulder.x - start.x, shoulder.y - start.y),
            vxs: 0.0,
            age: 0.0,
            feet,
            vel: v(0.0, 0.0),
        }
    }

    pub fn falling(&self) -> bool {
        self.phase == Phase::Fall
    }

    pub fn update(&mut self, dt: f32, cursor: V) {
        let dt = dt.clamp(0.001, 0.05);
        self.age += dt;
        let raw = (cursor.x - self.hand.x) / dt;
        self.hand = cursor;
        let prev = self.vxs;
        self.vxs += (raw - self.vxs) * 0.35;
        let ax = ((self.vxs - prev) / dt).clamp(-20000.0, 20000.0);
        // miring searah angin: kursor ke kanan -> badan condong ke kiri, makin cepat makin miring
        let lean = (-self.vxs / 2500.0).clamp(-0.8, 0.8);
        for _ in 0..2 {
            let h = dt / 2.0;
            let a =
                -(GRAV / PEND) * (self.th - lean).sin() - ax / PEND * self.th.cos() - 2.2 * self.w;
            self.w += a * h;
            self.th = (self.th + self.w * h).clamp(-1.5, 1.5);
        }
        if self.phase == Phase::Climb {
            self.p += dt / CLIMB_T;
            if self.p >= 1.0 {
                self.p = 1.0;
                self.phase = Phase::Hang;
            }
        }
    }

    /// Lepas: mulai jatuh dari posisi telapak kaki saat ini, membawa momentum ayunan.
    pub fn release(&mut self) {
        let q = self.pose();
        let f = |p: &(V, V)| p.1;
        let (a, b) = (f(&q.legs[0]), f(&q.legs[1]));
        self.feet = v(
            self.hand.x + (a.x + b.x) / 2.0,
            self.hand.y + (a.y + b.y) / 2.0,
        );
        let tang = self.w * PEND;
        self.vel = v(
            (self.vxs * 0.5 + tang * self.th.cos()).clamp(-700.0, 700.0),
            (-tang * self.th.sin()).clamp(-400.0, 400.0),
        );
        self.phase = Phase::Fall;
    }

    /// Satu langkah jatuh; true bila sudah mencapai `floor_y` (layar).
    pub fn fall(&mut self, dt: f32, floor_y: f32) -> bool {
        let dt = dt.clamp(0.001, 0.05);
        self.vel.y += GRAV * dt;
        self.vel.x *= 1.0 - 1.5 * dt;
        self.feet.x += self.vel.x * dt;
        self.feet.y += self.vel.y * dt;
        if self.feet.y >= floor_y {
            self.feet.y = floor_y;
            return true;
        }
        false
    }

    /// Pose relatif terhadap tangan (kursor) di (0,0).
    pub fn pose(&self) -> Pose {
        let f = self.face;
        let climb = self.phase == Phase::Climb;
        let pc = self.p;
        let d = v(self.th.sin(), self.th.cos());
        let dist = if climb {
            let k = ease(pc);
            (ARM + REACH_IN) * (1.0 - k) + ARM * k
        } else {
            ARM
        };
        let off = 1.0 - ease(pc / 0.35);
        let sh = v(
            d.x * dist + self.delta.x * off,
            d.y * dist + self.delta.y * off,
        );
        let hip = v(sh.x + d.x * TORSO, sh.y + d.y * TORSO);
        let head = v(sh.x - d.x * (NECK + HEADR), sh.y - d.y * (NECK + HEADR));
        // tangan: genggam tali bergantian saat memanjat, lalu menggenggam kursor
        let bl = ease((pc - 0.8) / 0.2);
        let arms = [0usize, 1].map(|i| {
            let side = if i == 0 { -2.5 } else { 2.5 };
            let hand = if climb {
                let u = (pc * CYCLES + i as f32 * 0.5) % 1.0;
                let r = (0.35 + 0.65 * ease(u)) * (1.0 - bl) + bl;
                let l = (r * (UA + FA) * 0.98).min(dist);
                v(sh.x - d.x * l + d.y * side, sh.y - d.y * l - d.x * side)
            } else {
                v(d.y * side, -d.x * side)
            };
            (ik(sh, hand, UA, FA, f), hand)
        });
        // kaki: menjuntai tertinggal dari ayunan; menekuk bergantian saat memanjat
        let lag = (-self.w * 0.05).clamp(-0.6, 0.6);
        let legs = [0usize, 1].map(|i| {
            let phi = self.th + lag + if i == 0 { 0.1 } else { -0.1 };
            let mut len = LEG * 0.97;
            if climb {
                len *= 0.8 + 0.12 * (TAU * (pc * CYCLES + i as f32 * 0.5)).sin();
            }
            let foot = v(hip.x + phi.sin() * len, hip.y + phi.cos() * len);
            (ik(hip, foot, TH, SH, -f), foot)
        });
        Pose {
            hip,
            sh,
            head,
            legs,
            arms,
        }
    }

    /// Sudut ayun (rad) untuk memutar sprite kustom di sekitar tangan.
    pub fn angle(&self) -> f32 {
        self.th
    }
}

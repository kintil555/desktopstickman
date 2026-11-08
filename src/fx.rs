//! Debu: port dari `Stickman Pixel.html` (spawnRun/spawnDust/drawDust). Satuan: px dunia; digambar di grid sel kanvas (GC).
use crate::stick::{Canvas, Rng};
use std::sync::OnceLock;

const C0: u32 = 0xFFFF_FFFF;
const C1: u32 = 0xFFF1_F1F4;
const C2: u32 = 0xFFDA_DADF;
const DSPR: &str = include_str!("../assets/dspr.txt");
const WALK: f32 = 350.0;

// DUST (jatuh)
const MIN_H: f32 = 24.0;
const MAX_H: f32 = 420.0;
const SPR_MIN: f32 = 0.2;

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Spr,
    Fleck,
    Tail,
    Roll,
}

pub struct Dust {
    kind: Kind,
    pub x: f32,
    dy: f32,
    vx: f32,
    vy: f32,
    dm: f32,
    life: f32,
    max: f32,
    r1: f32,
    sd: f32,
    dir: f32,
    spin: f32,
    sc: f32,
    scx: f32,
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn jsround(v: f32) -> i32 {
    (v + 0.5).floor() as i32
}

fn hash(x: i32, y: i32) -> f32 {
    let mut h = x
        .wrapping_mul(374_761_393)
        .wrapping_add(y.wrapping_mul(668_265_263));
    h = (h ^ ((h as u32) >> 13) as i32).wrapping_mul(1_274_126_177);
    ((h ^ ((h as u32) >> 16) as i32) as u32) as f32 / 4_294_967_296.0
}

fn base(kind: Kind, x: f32, dir: f32, life: f32) -> Dust {
    Dust {
        kind,
        x,
        dy: 0.0,
        vx: 0.0,
        vy: 0.0,
        dm: 3.0,
        life,
        max: life,
        r1: 1.0,
        sd: 0.0,
        dir,
        spin: 0.0,
        sc: 1.0,
        scx: 1.0,
    }
}

fn cap(l: &mut Vec<Dust>) {
    if l.len() > 140 {
        l.drain(..l.len() - 140);
    }
}

/// Kekuatan dari tinggi jatuh (px dunia).
pub fn dust_k(h: f32) -> f32 {
    ((h - MIN_H) / (MAX_H - MIN_H)).clamp(0.0, 1.0).powf(0.75)
}

/// Debu lari: gelombang sprite ke belakang kaki. `mv` = arah lari (+1/-1), `sp` = kecepatan.
pub fn spawn_run(l: &mut Vec<Dust>, x: f32, mv: f32, sp: f32, rng: &mut Rng) {
    let k = (((sp / WALK).min(1.0) * WALK - 180.0) / (WALK - 180.0))
        .clamp(0.0, 1.0)
        .powf(1.6);
    let dir = -mv;
    let life = lerp(0.26, 0.34, rng.f());
    let j = 0.92 + rng.f() * 0.16;
    let mut d = base(Kind::Spr, x + dir * 2.0, dir, life);
    d.vx = dir * lerp(12.0, 28.0, k);
    d.scx = lerp(0.5, 1.4, k) * j;
    d.sc = lerp(0.22, 0.65, k) * j;
    l.push(d);
    if rng.f() < 0.06 {
        let life = 0.25 + 0.3 * rng.f();
        let mut f = base(Kind::Fleck, x + dir * (14.0 + rng.f() * 40.0), dir, life);
        f.vx = dir * (10.0 + rng.f() * 20.0);
        f.vy = -(12.0 + rng.f() * 22.0);
        f.dm = 2.0;
        l.push(f);
    }
    cap(l);
}

/// Debu jatuh/mendarat, `k` 0..1 (lihat `dust_k`).
pub fn spawn_land(l: &mut Vec<Dust>, x: f32, k: f32, rng: &mut Rng) {
    let nr = 1 + (k > 0.3) as i32 + (k > 0.65) as i32;
    let ks = k.powf(1.6);
    for dir in [-1.0f32, 1.0] {
        if k >= SPR_MIN {
            let kk = (k - SPR_MIN) / (1.0 - SPR_MIN);
            let mut d = base(Kind::Spr, x + dir * 2.0, dir, 0.4 + 0.5 * kk);
            d.vx = dir * (20.0 + 46.0 * kk);
            d.sc = lerp(0.6, 1.8, kk);
            d.scx = d.sc;
            l.push(d);
        }
        for i in 1..=nr {
            let f = 1.0 - i as f32 * 0.32;
            let life = lerp(0.18, 0.68, ks) * (1.0 - i as f32 * 0.12);
            let mut d = base(Kind::Roll, x + dir * (3.0 + i as f32 * 2.0), dir, life);
            d.vx = dir * lerp(30.0, 300.0, ks) * f * (0.9 + rng.f() * 0.2);
            d.dm = 3.4 + i as f32 * 0.8;
            d.r1 = lerp(1.1, 9.1, ks) * f;
            d.sd = rng.f() * std::f32::consts::TAU;
            d.spin = 7.0 + rng.f() * 3.0;
            l.push(d);
        }
        for _ in 0..jsround(lerp(1.0, 6.0, ks)) {
            let life = (0.14 + 0.28 * ks) * (0.7 + rng.f() * 0.6);
            let mut d = base(
                Kind::Tail,
                x + dir * rng.f() * (14.0 + 76.0 * ks),
                dir,
                life,
            );
            d.vx = dir * (8.0 + rng.f() * (10.0 + 30.0 * ks));
            d.vy = -(3.0 + rng.f() * (5.0 + 7.0 * ks));
            d.dm = 2.0;
            d.r1 = 0.9 + rng.f() * (0.8 + 2.9 * ks);
            l.push(d);
        }
    }
    cap(l);
}

pub fn update(l: &mut Vec<Dust>, dt: f32) {
    for q in l.iter_mut() {
        q.x += q.vx * dt;
        q.dy += q.vy * dt;
        q.vx *= (-q.dm * dt).exp();
        q.vy = q.vy * (-3.0 * dt).exp() - 14.0 * dt;
        q.life -= dt;
    }
    l.retain(|q| q.life > 0.0);
}

type Field = (Vec<f32>, Vec<f32>);

fn fields() -> &'static Vec<Field> {
    static F: OnceLock<Vec<Field>> = OnceLock::new();
    F.get_or_init(|| {
        DSPR.lines()
            .map(|ln| {
                let b = ln.as_bytes();
                let o = (0..384).map(|i| (b.get(i).copied().unwrap_or(b'.') != b'.') as u8 as f32);
                let h = (0..384).map(|i| (b.get(i).copied() == Some(b'W')) as u8 as f32);
                (o.collect(), h.collect())
            })
            .collect()
    })
}

fn samp(a: &[f32], x: f32, y: f32) -> f32 {
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let g = |i: f32, j: f32| {
        if i < 0.0 || j < 0.0 || i > 23.0 || j > 15.0 {
            0.0
        } else {
            a[j as usize * 24 + i as usize]
        }
    };
    g(x0, y0) * (1.0 - fx) * (1.0 - fy)
        + g(x0 + 1.0, y0) * fx * (1.0 - fy)
        + g(x0, y0 + 1.0) * (1.0 - fx) * fy
        + g(x0 + 1.0, y0 + 1.0) * fx * fy
}

struct Ctx<'a> {
    c: &'a mut Canvas,
    gy: i32,
    xo: i32,
}

impl Ctx<'_> {
    fn put(&mut self, x: i32, y: i32, col: u32) {
        if x >= 0 && y >= 0 && (x as usize) < self.c.w && (y as usize) < self.c.h && y <= self.gy {
            let w = self.c.w;
            self.c.px[y as usize * w + x as usize] = col;
        }
    }
    /// hash stabil di dunia (tidak berkedip saat jendela bergeser).
    fn hs(&self, x: i32, y: i32) -> f32 {
        hash(x + self.xo, y - self.gy + 1000)
    }
}

/// Gambar semua debu. `wx` = x dunia yang berada di `ox` (x kanvas, px); `floor_y` = garis lantai di kanvas (px).
pub fn draw_all(c: &mut Canvas, list: &[Dust], wx: f32, ox: f32, floor_y: f32) {
    let cell = c.cell;
    let (gy, xo) = (
        (floor_y / cell).floor() as i32,
        ((wx - ox) / cell).floor() as i32,
    );
    let lwg = c.w as i32;
    let mut cx_ = Ctx { c, gy, xo };
    for q in list {
        let t = 1.0 - q.life / q.max;
        let cx = ((q.x - wx + ox) / cell).floor() as i32;
        if !(-40..=lwg + 40).contains(&cx) {
            continue;
        }
        let th = if t < 0.45 {
            1.0
        } else {
            (1.0 - (t - 0.45) / 0.55).max(0.0)
        };
        let rise = q.dy / cell;
        let e = (t / 0.3).min(1.0);
        let r = q.r1 * (0.3 + 0.7 * (1.0 - (1.0 - e) * (1.0 - e) * (1.0 - e)));
        if th <= 0.0 {
            continue;
        }
        match q.kind {
            Kind::Fleck => {
                let yb = gy + jsround(rise);
                if t < 0.6 || cx_.hs(cx, yb) > 0.5 {
                    cx_.put(cx, yb, C0);
                }
            }
            Kind::Spr => draw_spr(&mut cx_, q, t, cx),
            Kind::Tail => {
                let w = jsround(r * 1.3 * th + 0.5);
                let h = if th > 0.45 { 2 } else { 1 };
                let yb = gy + jsround(rise);
                for j in 0..h {
                    for xx in -w..=w {
                        cx_.put(cx + xx, yb - j, if j == h - 1 && h > 1 { C0 } else { C1 });
                    }
                }
            }
            Kind::Roll => draw_roll(&mut cx_, q, t, cx, r, th, rise),
        }
    }
}

fn draw_spr(cx_: &mut Ctx, q: &Dust, t: f32, cx: i32) {
    let fs = fields();
    let fi = ((t.powf(0.85) * 11.0).floor() as usize).min(10);
    let Some(fd) = fs.get(fi) else { return };
    let (sc, sx) = (q.sc, q.scx);
    let w = (24.0 * sx).ceil() as i32 + 1;
    let amp = 0.1 + 0.2 * (sc - 1.0).clamp(0.0, 1.0);
    let gy = cx_.gy;
    for oy in (-((16.0 * sc).ceil() as i32) - 1)..=0 {
        let y = gy + oy;
        let iy = 15.0 - (-oy) as f32 / sc;
        if !(-0.5..=15.5).contains(&iy) {
            continue;
        }
        for ox2 in -w..=w {
            let x = cx + ox2;
            let ix = if q.dir < 0.0 {
                (ox2 as f32 + 0.5) / sx + 21.5
            } else {
                22.5 - (ox2 as f32 + 0.5) / sx
            };
            if !(-0.5..=23.5).contains(&ix) {
                continue;
            }
            if samp(&fd.0, ix, iy) < 0.5 + (cx_.hs(x, (y - gy) * 3 + 7 + gy) - 0.5) * amp * 2.0 {
                continue;
            }
            let col = if samp(&fd.1, ix, iy) >= 0.5 {
                C0
            } else if iy >= 13.5 || oy > -1 || (-oy) as f32 / (16.0 * sc) < 0.1 {
                C2
            } else {
                C1
            };
            cx_.put(x, y, col);
        }
    }
}

fn draw_roll(cx_: &mut Ctx, q: &Dust, t: f32, cx: i32, r: f32, th: f32, rise: f32) {
    let sp = q.sd + q.dir * q.spin * t;
    let n = ((r * 3.6).round() as i32).max(8);
    let l = 0.3 + 0.7 * th;
    let cyc = -r * 0.6 + rise;
    let gy = cx_.gy;
    for i in (0..=n).rev() {
        let s = i as f32 / n as f32 * l;
        let an = sp + q.dir * s * 6.9;
        let ro = r * (0.1 + 0.75 * s);
        let rb = r * 0.4 * (1.0 - 0.5 * s) * (0.3 + 0.7 * th);
        if rb < 0.5 {
            continue;
        }
        let sx = cx + jsround(an.cos() * ro);
        let sy = gy + jsround(an.sin() * ro * 0.8 + cyc);
        let ri = rb.ceil() as i32;
        let r2 = rb * rb;
        for yy in -ri..=ri {
            let y = sy + yy;
            if y > gy {
                continue;
            }
            for xx in -ri..=ri {
                if (xx * xx + yy * yy) as f32 > r2 {
                    continue;
                }
                let x = sx + xx;
                let mut lt: f32 = (-xx as f32 * 0.55 - yy as f32 * 0.7) / rb.max(1.0)
                    + (cx_.hs(x, y) - 0.5) * 0.12
                    - 0.12 * s;
                lt -= 0.3 * (((y - gy) as f32 + r * 0.9) / (r * 0.9)).clamp(0.0, 1.0);
                cx_.put(
                    x,
                    y,
                    if lt > 0.15 {
                        C0
                    } else if lt > -0.3 {
                        C1
                    } else {
                        C2
                    },
                );
            }
        }
    }
}

//! Kendaraan terbang TSR P400: sprite 59x59 (1 piksel sprite = 2px dunia), api prosedural.
use crate::{
    config::Config,
    fx,
    skin::{self, Sheet},
    stick::{Canvas, Pose, Stick, V},
};

const BODY: &[u8] = include_bytes!("../assets/tsr_body.rgba");
const JB: &[u8] = include_bytes!("../assets/tsr_jb.rgba");
const JF: &[u8] = include_bytes!("../assets/tsr_jf.rgba");
const NZ: [[f32; 2]; 2] = [[11.5, 53.0], [48.0, 52.0]];
const CLIP: [[f32; 2]; 10] = [
    [25.0, 4.0],
    [33.0, 4.0],
    [37.0, 8.0],
    [40.0, 14.0],
    [42.0, 22.0],
    [42.0, 28.0],
    [20.0, 28.0],
    [20.0, 20.0],
    [21.0, 14.0],
    [22.0, 8.0],
];
pub const PC: f32 = 2.0;

/// Offset ujung nozzle `j` dari titik pijak kendaraan (px dunia), dengan mirror untuk f<0.
pub fn nozzle(j: usize, f: f32) -> V {
    g(NZ[j][0], NZ[j][1], f)
}

/// Koordinat sprite (grid) -> offset dunia dari titik pijak, dengan mirror untuk f<0.
fn g(x: f32, y: f32, f: f32) -> V {
    V {
        x: if f > 0.0 { x - 30.0 } else { 29.0 - x } * PC,
        y: (y - 54.0) * PC,
    }
}

pub fn seated(f: f32) -> Pose {
    Pose {
        hip: g(28.0, 27.0, f),
        sh: g(27.5, 15.6, f),
        head: g(27.2, 9.6, f),
        legs: [
            (g(36.5, 26.5, f), g(38.5, 33.0, f)),
            (g(35.5, 27.5, f), g(37.0, 34.0, f)),
        ],
        arms: [
            (g(33.0, 19.5, f), g(36.0, 22.0, f)),
            (g(31.5, 22.0, f), g(34.0, 26.0, f)),
        ],
    }
}

fn blit(c: &mut Canvas, spr: &[u8], o: V, f: f32) {
    for gy in 0..c.h {
        for gx in 0..c.w {
            let (px, py) = (
                (gx as f32 + 0.5) * c.cell - o.x,
                (gy as f32 + 0.5) * c.cell - o.y,
            );
            let (ix, iy) = ((px / PC).floor(), (py / PC).floor());
            let sx = if f > 0.0 { ix + 30.0 } else { 29.0 - ix };
            let sy = iy + 54.0;
            if !(0.0..59.0).contains(&sx) || !(0.0..59.0).contains(&sy) {
                continue;
            }
            let q = (sy as usize * 59 + sx as usize) * 4;
            if spr[q + 3] >= 128 {
                c.px[gy * c.w + gx] = 0xFF00_0000
                    | (spr[q] as u32) << 16
                    | (spr[q + 1] as u32) << 8
                    | spr[q + 2] as u32;
            }
        }
    }
}

fn flame(c: &mut Canvas, s: &Stick, o: V, floor_y: f32) {
    const HOT: u32 = 0xFFEA_FDFF;
    const BR: u32 = 0xFFAD_FFFC;
    const CY: u32 = 0xFF3F_DFF2;
    const BL: u32 = 0xFF5E_BAFA;
    const VI: u32 = 0xFF78_5FD9;
    let (f, thr, tm) = (s.face, s.thr, s.t);
    for (j, n) in NZ.iter().enumerate() {
        let nz = g(n[0], n[1], f);
        let jf = j as f32;
        let l = (5.0 + 9.0 * thr) * (0.86 + 0.14 * (tm * 38.0 + jf * 2.3).sin());
        let wh = 3.4 + 0.9 * thr;
        for gy in 0..c.h {
            let py = (gy as f32 + 0.5) * c.cell;
            if py > floor_y {
                continue;
            }
            for gx in 0..c.w {
                let rx = ((gx as f32 + 0.5) * c.cell - o.x - nz.x) / PC;
                let ry = ((py - o.y) - nz.y) / PC;
                if ry < 0.0 || ry > l {
                    continue;
                }
                let t = ry / l;
                let h = wh
                    * (1.0 - t.powf(1.5))
                    * (1.0 + 0.12 * (ry * 2.7 - tm * 45.0 + jf * 3.0).sin());
                let a = rx.abs();
                if a > h {
                    continue;
                }
                let r = a / h;
                c.px[gy * c.w + gx] = if t > 0.85 {
                    BL
                } else if r < 0.22 && t < 0.5 {
                    HOT
                } else if r < 0.5 && t < 0.7 {
                    BR
                } else if r < 0.75 {
                    CY
                } else if r < 0.92 {
                    BL
                } else {
                    VI
                };
            }
        }
        // glow additive di sepanjang sumbu api: terang di pangkal, mengecil ke ujung
        let gk = 0.85 + 0.15 * (tm * 29.0 + jf * 2.1).sin();
        let gr = 6.0 + 5.0 * thr;
        let glows = [
            (0.12, gr, [173, 255, 252], 0.45),
            (0.4, gr * 0.85, [63, 223, 242], 0.4),
            (0.72, gr * 0.6, [94, 186, 250], 0.3),
        ];
        for (u, r, col, k) in glows {
            let p = V {
                x: o.x + nz.x,
                y: o.y + nz.y + l * PC * u,
            };
            glow(c, p, r * PC, col, k * gk, floor_y);
        }
    }
}

/// Cahaya additive profil (1-d/r)^2, dipotong di garis lantai.
fn glow(c: &mut Canvas, p: V, r: f32, col: [u8; 3], k: f32, floor_y: f32) {
    let cell = c.cell;
    let x0 = ((p.x - r) / cell).floor().max(0.0) as usize;
    let y0 = ((p.y - r) / cell).floor().max(0.0) as usize;
    let x1 = (((p.x + r) / cell).ceil().max(0.0) as usize).min(c.w.saturating_sub(1));
    let y1 = (((p.y + r) / cell).ceil().max(0.0) as usize).min(c.h.saturating_sub(1));
    for gy in y0..=y1 {
        let py = (gy as f32 + 0.5) * cell;
        if py > floor_y {
            continue;
        }
        for gx in x0..=x1 {
            let (dx, dy) = ((gx as f32 + 0.5) * cell - p.x, py - p.y);
            let t = dx.hypot(dy) / r;
            if t < 1.0 {
                fx::add(&mut c.px[gy * c.w + gx], col, (1.0 - t) * (1.0 - t) * k);
            }
        }
    }
}

fn inside(p: V, poly: &[V]) -> bool {
    let mut ins = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            ins = !ins;
        }
        j = i;
    }
    ins
}

/// `o` = titik pijak kendaraan di kanvas; `floor_y` = garis lantai (api dipotong di sini).
/// Dengan `sheet` (skin kustom) pemain digambar tanpa clip kokpit, skala `seat_scale`.
pub fn draw(c: &mut Canvas, s: &Stick, o: V, cfg: &Config, floor_y: f32, sheet: Option<&Sheet>) {
    flame(c, s, o, floor_y);
    blit(c, JB, o, s.face);
    blit(c, BODY, o, s.face);
    if let Some(sh) = sheet {
        let q = g(28.0, 34.0, s.face);
        let seat = V {
            x: o.x + q.x,
            y: o.y + q.y,
        };
        skin::draw_player(c, sh, cfg, s, seat, true, None);
    } else {
        let mut tmp = Canvas::new(c.w as u32 * cfg.cell, c.h as u32 * cfg.cell, cfg.cell);
        tmp.draw(&seated(s.face), o, cfg);
        let poly: Vec<V> = CLIP
            .iter()
            .map(|p| {
                let q = g(p[0], p[1], s.face);
                V {
                    x: o.x + q.x,
                    y: o.y + q.y,
                }
            })
            .collect();
        for gy in 0..c.h {
            for gx in 0..c.w {
                let i = gy * c.w + gx;
                let ctr = V {
                    x: (gx as f32 + 0.5) * c.cell,
                    y: (gy as f32 + 0.5) * c.cell,
                };
                if tmp.px[i] != 0 && inside(ctr, &poly) {
                    c.px[i] = tmp.px[i];
                }
            }
        }
    }
    blit(c, JF, o, s.face);
}

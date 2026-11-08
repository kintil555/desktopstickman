//! Item yang dipegang stickman: PNG pixel art digambar tanpa antialias (nearest neighbor).
use crate::{
    config::Item,
    skin::{self, Sheet},
    stick::{Canvas, Pose, V},
};
use std::collections::HashMap;

/// Cache gambar item per path (None = gagal dimuat).
pub type Arts = HashMap<String, Option<Sheet>>;

#[derive(Clone, Copy)]
pub struct Draw<'a> {
    pub item: &'a Item,
    pub art: &'a Sheet,
}

/// Muat gambar yang belum ada di cache.
pub fn ensure(arts: &mut Arts, items: &[Item]) {
    for it in items {
        if !arts.contains_key(&it.path) {
            arts.insert(it.path.clone(), skin::load(&it.path));
        }
    }
}

/// Buang gambar yang tidak dipakai lagi, lalu muat yang baru.
pub fn sync(arts: &mut Arts, items: &[Item]) {
    arts.retain(|k, _| items.iter().any(|i| i.path == *k));
    ensure(arts, items);
}

/// Item `id` beserta gambarnya (None = tangan kosong atau gambar gagal dimuat).
pub fn pick<'a>(items: &'a [Item], id: u32, arts: &'a Arts) -> Option<Draw<'a>> {
    let item = items.iter().find(|i| i.id == id)?;
    let art = arts.get(&item.path)?.as_ref()?;
    Some(Draw { item, art })
}

/// Item berikutnya dalam siklus: kosong -> item 1 -> item 2 -> ... -> kosong.
pub fn next(items: &[Item], cur: u32) -> u32 {
    match items.iter().position(|i| i.id == cur) {
        None => items.first().map_or(0, |i| i.id),
        Some(p) => items.get(p + 1).map_or(0, |i| i.id),
    }
}

/// Gambar item di node tangan. Tiap sel kanvas mengambil satu piksel gambar (tanpa blur),
/// piksel dengan alpha < 128 dilewati; yang lain digambar penuh.
pub fn draw(c: &mut Canvas, d: Draw, q: &Pose, o: V, face: f32) {
    let (it, art) = (d.item, d.art);
    if art.w == 0 || art.h == 0 {
        return;
    }
    let (elbow, hand) = q.arms[(it.hand as usize).min(1)];
    let m = if it.flip && face < 0.0 { -1.0 } else { 1.0 };
    // arah lengan bawah (siku -> tangan): 0 = ke atas, searah jarum jam positif
    let follow = if it.follow_arm {
        (hand.x - elbow.x).atan2(elbow.y - hand.y)
    } else {
        0.0
    };
    let (sn, cs) = (follow + it.angle.to_radians() * m).sin_cos();
    let anchor = V {
        x: o.x + hand.x + it.off_x * m,
        y: o.y + hand.y + it.off_y,
    };
    let cell = c.cell;
    // px layar per piksel gambar
    let px = (it.scale * cell).max(0.01);
    let (w, h) = (art.w as f32, art.h as f32);
    let rad = w.hypot(h) * px;
    let x0 = ((anchor.x - rad) / cell).floor().max(0.0) as usize;
    let y0 = ((anchor.y - rad) / cell).floor().max(0.0) as usize;
    let x1 = (((anchor.x + rad) / cell).ceil().max(0.0) as usize).min(c.w);
    let y1 = (((anchor.y + rad) / cell).ceil().max(0.0) as usize).min(c.h);
    for cy in y0..y1 {
        for cx in x0..x1 {
            let dx = (cx as f32 + 0.5) * cell - anchor.x;
            let dy = (cy as f32 + 0.5) * cell - anchor.y;
            // putar balik ke ruang gambar, lalu ke piksel gambar (titik genggam = pivot)
            let sx = dx * cs + dy * sn;
            let sy = dy * cs - dx * sn;
            let ix = (sx * m / px + it.pivot_x * w).floor();
            let iy = (sy / px + it.pivot_y * h).floor();
            if ix < 0.0 || iy < 0.0 || ix >= w || iy >= h {
                continue;
            }
            let i = (iy as usize * art.w + ix as usize) * 4;
            let p = &art.px[i..i + 4];
            if p[3] >= 128 {
                c.px[cy * c.w + cx] = u32::from_be_bytes([255, p[0], p[1], p[2]]);
            }
        }
    }
}

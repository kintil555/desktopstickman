use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, time::SystemTime};

pub fn path() -> PathBuf {
    crate::sys::dir().join("desktopstickman.json")
}

/// Satu baris sprite sheet: baris ke-`row`, `frames` frame, `fps` frame per detik.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Anim {
    pub row: u32,
    pub frames: u32,
    pub fps: u32,
}

impl Anim {
    const fn new(row: u32, frames: u32, fps: u32) -> Self {
        Self { row, frames, fps }
    }
}

impl Default for Anim {
    fn default() -> Self {
        Self::new(0, 1, 6)
    }
}

/// Model pemain kustom dari sprite sheet PNG (grid `cols` x `rows`, frame sama besar).
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Skin {
    pub enabled: bool,
    pub path: String,
    pub cols: u32,
    pub rows: u32,
    /// Ukuran satu piksel sprite di layar (px).
    pub scale: f32,
    /// Pengali `scale` saat duduk di kendaraan.
    pub seat_scale: f32,
    /// Geser sprite (satuan piksel sprite); x ikut arah hadap.
    pub off_x: i32,
    pub off_y: i32,
    /// Sprite aslinya menghadap kiri.
    pub faces_left: bool,
    /// Kalikan warna sprite dengan warna model.
    pub tint: bool,
    pub idle: Anim,
    pub walk: Anim,
    pub run: Anim,
    pub jump: Anim,
}

impl Default for Skin {
    fn default() -> Self {
        Self {
            enabled: false,
            path: String::new(),
            cols: 4,
            rows: 4,
            scale: 2.0,
            seat_scale: 0.6,
            off_x: 0,
            off_y: 0,
            faces_left: false,
            tint: false,
            idle: Anim::new(0, 4, 4),
            walk: Anim::new(1, 4, 8),
            run: Anim::new(2, 4, 12),
            jump: Anim::new(3, 1, 1),
        }
    }
}

/// Bentuk sosok prosedural (tab Model di panel).
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Body {
    /// Kepala terisi penuh (bukan cincin kosong).
    pub head_fill: bool,
    pub head_size: f32,
    /// Tebal garis = setengah lebar, px kanvas.
    pub head_w: f32,
    pub limb_w: f32,
    pub torso_w: f32,
    pub outline: bool,
    pub outline_color: [u8; 3],
    pub shade: bool,
    /// Kekuatan highlight/shadow 0..2.
    pub shade_k: f32,
    /// Terang warna anggota badan belakang relatif warna depan.
    pub back_k: f32,
}

impl Default for Body {
    fn default() -> Self {
        Self {
            head_fill: false,
            head_size: 1.0,
            head_w: 2.0,
            limb_w: 2.0,
            torso_w: 2.0,
            outline: true,
            outline_color: [40, 20, 10],
            shade: true,
            shade_k: 1.0,
            back_k: 0.75,
        }
    }
}

/// Tombol pintas global (Windows). Nama tombol, kosong = nonaktif.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Keys {
    pub hang: String,
    pub jump: String,
    pub fly: String,
    pub hide: String,
    /// Ganti item yang dipegang (siklus).
    pub item: String,
}

impl Default for Keys {
    fn default() -> Self {
        Self {
            hang: "Alt+C".into(),
            jump: String::new(),
            fly: String::new(),
            hide: String::new(),
            item: String::new(),
        }
    }
}

/// Item yang dipegang stickman: gambar PNG (pixel art, tanpa antialias) di node tangan.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Item {
    pub id: u32,
    pub name: String,
    pub path: String,
    /// 0 = tangan depan, 1 = tangan belakang.
    pub hand: u32,
    /// Titik genggam di gambar (0..1 dari lebar/tinggi); titik ini menempel di tangan.
    pub pivot_x: f32,
    pub pivot_y: f32,
    /// Geser dari node tangan (px layar); x ikut arah hadap.
    pub off_x: f32,
    pub off_y: f32,
    /// Piksel kanvas per piksel gambar.
    pub scale: f32,
    /// Putar (derajat, searah jarum jam).
    pub angle: f32,
    /// Ikut arah lengan bawah (siku -> tangan).
    pub follow_arm: bool,
    /// Gambar dibalik saat stickman menghadap kiri.
    pub flip: bool,
}

impl Default for Item {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            path: String::new(),
            hand: 0,
            pivot_x: 0.15,
            pivot_y: 0.85,
            off_x: 0.0,
            off_y: 0.0,
            scale: 1.0,
            angle: 0.0,
            follow_arm: false,
            flip: true,
        }
    }
}

impl Item {
    fn sanitize(&mut self) {
        self.hand = self.hand.min(1);
        self.pivot_x = self.pivot_x.clamp(0.0, 1.0);
        self.pivot_y = self.pivot_y.clamp(0.0, 1.0);
        self.off_x = self.off_x.clamp(-100.0, 100.0);
        self.off_y = self.off_y.clamp(-100.0, 100.0);
        self.scale = self.scale.clamp(0.25, 8.0);
        self.angle = self.angle.clamp(-360.0, 360.0);
    }
}

/// Satu stickman di daftar, dengan pengaturan sendiri (warna, model, shadow, skin, ...).
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Inst {
    pub id: u32,
    /// Pengaturan khas stickman ini; hanya bagian `Config::copy_look` yang dipakai.
    pub cfg: Config,
}

impl Inst {
    /// Salin tampilan/perilaku dari `c` sebagai nilai awal.
    pub fn from_cfg(id: u32, c: &Config) -> Self {
        let mut cfg = c.clone();
        cfg.stickmen.clear();
        cfg.items.clear();
        Self { id, cfg }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Config {
    /// Ukuran sel piksel (px layar per sel).
    pub cell: u32,
    /// Pengali kecepatan jalan.
    pub speed: f32,
    /// Jarak lantai dari tepi bawah monitor (tinggi taskbar).
    pub floor_offset: i32,
    /// Satu warna model; warna badan belakang diturunkan otomatis (`back`).
    #[serde(alias = "color_f")]
    pub color: [u8; 3],
    pub jump: bool,
    pub run: bool,
    pub fly: bool,
    pub dust: bool,
    /// Klik kanan pada stickman = bergantung di kursor mouse (Windows).
    pub hang: bool,
    pub sound: bool,
    pub volume: f32,
    pub shadow: bool,
    /// 0..1
    pub shadow_opacity: f32,
    /// Geser bayangan (px layar).
    pub shadow_dx: i32,
    pub shadow_dy: i32,
    /// Radius blur dalam sel (0 = tepi tajam).
    pub shadow_blur: u32,
    pub shadow_color: [u8; 3],
    /// Jalan otomatis saat login (`--startup`) setelah `startup_delay` detik.
    pub autostart: bool,
    pub startup_delay: u32,
    /// Overlay tetap jalan setelah panel ditutup.
    pub bg_on_close: bool,
    pub skin: Skin,
    pub body: Body,
    pub keys: Keys,
    /// Daftar stickman; kosong = satu stickman memakai pengaturan global.
    pub stickmen: Vec<Inst>,
    /// Pustaka item (dipakai bersama); `held` = item yang dipegang (0 = tangan kosong).
    pub items: Vec<Item>,
    pub held: u32,
    /// Stickman saling serang (global); `hp` dan `dmg` per stickman.
    pub fight: bool,
    pub hp: u32,
    pub dmg: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cell: 2,
            speed: 1.0,
            floor_offset: 48,
            color: [238, 122, 28],
            jump: true,
            run: true,
            fly: true,
            dust: true,
            hang: true,
            sound: true,
            volume: 0.6,
            shadow: true,
            shadow_opacity: 0.4,
            shadow_dx: 4,
            shadow_dy: 4,
            shadow_blur: 0,
            shadow_color: [0, 0, 0],
            autostart: false,
            startup_delay: 30,
            bg_on_close: true,
            skin: Skin::default(),
            body: Body::default(),
            keys: Keys::default(),
            stickmen: Vec::new(),
            items: Vec::new(),
            held: 0,
            fight: false,
            hp: 5,
            dmg: 1,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let mut c: Config = fs::read_to_string(path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        c.sanitize();
        c
    }

    pub fn next_item_id(&self) -> u32 {
        self.items.iter().map(|i| i.id).max().unwrap_or(0) + 1
    }

    pub fn next_id(&self) -> u32 {
        self.stickmen.iter().map(|i| i.id).max().unwrap_or(0) + 1
    }

    /// Timpa pengaturan global dengan milik stickman `id`; false = id tidak ada di daftar.
    pub fn apply_inst(&mut self, id: u32) -> bool {
        if id == 0 {
            return true;
        }
        let Some(inst) = self.stickmen.iter().find(|i| i.id == id) else {
            return false;
        };
        let look = inst.cfg.clone();
        self.copy_look(&look);
        true
    }

    /// Pengaturan per stickman (sisanya global: tombol, lantai, autostart, suara, ...).
    pub fn copy_look(&mut self, o: &Config) {
        self.cell = o.cell;
        self.speed = o.speed;
        self.color = o.color;
        self.jump = o.jump;
        self.run = o.run;
        self.fly = o.fly;
        self.dust = o.dust;
        self.shadow = o.shadow;
        self.shadow_opacity = o.shadow_opacity;
        self.shadow_dx = o.shadow_dx;
        self.shadow_dy = o.shadow_dy;
        self.shadow_blur = o.shadow_blur;
        self.shadow_color = o.shadow_color;
        self.skin = o.skin.clone();
        self.body = o.body.clone();
        self.held = o.held;
        self.hp = o.hp;
        self.dmg = o.dmg;
    }

    pub fn sanitize(&mut self) {
        for i in &mut self.stickmen {
            i.cfg.sanitize();
        }
        for it in &mut self.items {
            it.sanitize();
        }
        self.hp = self.hp.clamp(1, 100);
        self.dmg = self.dmg.clamp(1, 100);
        self.cell = self.cell.clamp(1, 8);
        self.speed = self.speed.clamp(0.1, 3.0);
        self.volume = self.volume.clamp(0.0, 1.0);
        self.shadow_opacity = self.shadow_opacity.clamp(0.0, 1.0);
        self.shadow_dx = self.shadow_dx.clamp(-32, 32);
        self.shadow_dy = self.shadow_dy.clamp(-32, 32);
        self.shadow_blur = self.shadow_blur.min(4);
        self.startup_delay = self.startup_delay.min(600);
        let b = &mut self.body;
        b.head_size = b.head_size.clamp(0.5, 2.0);
        b.head_w = b.head_w.clamp(1.0, 6.0);
        b.limb_w = b.limb_w.clamp(1.0, 6.0);
        b.torso_w = b.torso_w.clamp(1.0, 6.0);
        b.shade_k = b.shade_k.clamp(0.0, 2.0);
        b.back_k = b.back_k.clamp(0.3, 1.0);
        for k in [
            &mut self.keys.hang,
            &mut self.keys.jump,
            &mut self.keys.fly,
            &mut self.keys.hide,
            &mut self.keys.item,
        ] {
            *k = k.trim().to_string();
            if crate::sys::parse(k).is_none() {
                k.clear();
            }
        }
        let k = &mut self.skin;
        k.cols = k.cols.clamp(1, 32);
        k.rows = k.rows.clamp(1, 32);
        k.scale = k.scale.clamp(0.5, 16.0);
        k.seat_scale = k.seat_scale.clamp(0.2, 2.0);
        k.off_x = k.off_x.clamp(-64, 64);
        k.off_y = k.off_y.clamp(-64, 64);
        let (cols, rows) = (k.cols, k.rows);
        for a in [&mut k.idle, &mut k.walk, &mut k.run, &mut k.jump] {
            a.row = a.row.min(rows - 1);
            a.frames = a.frames.clamp(1, cols);
            a.fps = a.fps.clamp(1, 60);
        }
    }

    pub fn save(&self) {
        if let Ok(s) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path(), s);
        }
    }

    pub fn mtime() -> Option<SystemTime> {
        fs::metadata(path()).and_then(|m| m.modified()).ok()
    }

    /// Warna badan belakang: turunan otomatis dari `color`.
    pub fn back(&self) -> [u8; 3] {
        self.color
            .map(|v| (v as f32 * self.body.back_k).round() as u8)
    }
}

pub fn to_hex(c: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2])
}

pub fn parse_hex(s: &str) -> Option<[u8; 3]> {
    let h = s.trim().trim_start_matches('#');
    if h.len() != 6 || !h.is_ascii() {
        return None;
    }
    let v = u32::from_str_radix(h, 16).ok()?;
    Some([(v >> 16) as u8, (v >> 8) as u8, v as u8])
}

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

    pub fn sanitize(&mut self) {
        self.cell = self.cell.clamp(1, 8);
        self.speed = self.speed.clamp(0.1, 3.0);
        self.volume = self.volume.clamp(0.0, 1.0);
        self.shadow_opacity = self.shadow_opacity.clamp(0.0, 1.0);
        self.shadow_dx = self.shadow_dx.clamp(-32, 32);
        self.shadow_dy = self.shadow_dy.clamp(-32, 32);
        self.shadow_blur = self.shadow_blur.min(4);
        self.startup_delay = self.startup_delay.min(600);
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
        self.color.map(|v| (v as f32 * 0.75).round() as u8)
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

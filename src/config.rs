use serde::{Deserialize, Serialize};
use std::{fs, time::SystemTime};

pub const PATH: &str = "desktopstickman.json";

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Config {
    /// Ukuran sel piksel (px layar per sel).
    pub cell: u32,
    /// Pengali kecepatan jalan.
    pub speed: f32,
    /// Jarak lantai dari tepi bawah monitor (tinggi taskbar).
    pub floor_offset: i32,
    pub color_f: [u8; 3],
    pub color_b: [u8; 3],
    pub jump: bool,
    pub run: bool,
    pub fly: bool,
    pub dust: bool,
    pub sound: bool,
    pub volume: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cell: 2,
            speed: 1.0,
            floor_offset: 48,
            color_f: [238, 122, 28],
            color_b: [184, 84, 26],
            jump: true,
            run: true,
            fly: true,
            dust: true,
            sound: true,
            volume: 0.6,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let mut c: Config = fs::read_to_string(PATH)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        c.cell = c.cell.clamp(1, 8);
        c.speed = c.speed.clamp(0.1, 3.0);
        c.volume = c.volume.clamp(0.0, 1.0);
        c
    }

    pub fn save(&self) {
        if let Ok(s) = serde_json::to_string_pretty(self) {
            let _ = fs::write(PATH, s);
        }
    }

    pub fn mtime() -> Option<SystemTime> {
        fs::metadata(PATH).and_then(|m| m.modified()).ok()
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

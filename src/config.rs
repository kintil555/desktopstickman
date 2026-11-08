use serde::Deserialize;
use std::{fs, time::SystemTime};

pub const PATH: &str = "desktopstickman.json";

#[derive(Deserialize, Clone, Debug)]
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
        c
    }

    pub fn mtime() -> Option<SystemTime> {
        fs::metadata(PATH).and_then(|m| m.modified()).ok()
    }
}

//! Utilitas sistem: pid overlay (proses terpisah), autostart, prioritas rendah.
use std::{
    fs,
    path::PathBuf,
    process::{Command, Stdio},
};

const NAME: &str = "desktopstickman";

/// Folder exe: config & pid disimpan di sini supaya sama untuk panel, overlay, dan autostart.
pub fn dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(PathBuf::from))
        .unwrap_or_default()
}

/// Batas jumlah stickman (satu proses overlay per stickman).
pub const MAX_SPAWN: usize = 12;

fn pid_file(pid: u32) -> PathBuf {
    dir().join(format!("{NAME}-{pid}.pid"))
}

pub fn write_pid() {
    let _ = fs::write(pid_file(std::process::id()), "");
}

pub fn clear_pid() {
    let _ = fs::remove_file(pid_file(std::process::id()));
}

#[cfg(windows)]
fn quiet(c: &mut Command) -> &mut Command {
    use std::os::windows::process::CommandExt;
    c.creation_flags(0x0800_0000) // CREATE_NO_WINDOW
}

#[cfg(not(windows))]
fn quiet(c: &mut Command) -> &mut Command {
    c
}

fn run(c: &mut Command) -> Option<String> {
    let o = quiet(c)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    Some(String::from_utf8_lossy(&o.stdout).to_lowercase())
}

/// Nama proses (huruf kecil, maks 15 karakter seperti `comm` di Unix).
fn exe_name() -> String {
    let n = std::env::current_exe()
        .ok()
        .and_then(|e| e.file_stem().map(|s| s.to_string_lossy().to_lowercase()))
        .unwrap_or_else(|| NAME.to_string());
    n.chars().take(15).collect()
}

fn alive(pid: u32) -> bool {
    let name = exe_name();
    let p = pid.to_string();
    #[cfg(windows)]
    let out = {
        let q = format!("PID eq {p}");
        run(Command::new("tasklist").args(["/FI", q.as_str(), "/NH", "/FO", "CSV"]))
    };
    #[cfg(not(windows))]
    let out = run(Command::new("ps").args(["-p", p.as_str(), "-o", "comm="]));
    out.is_some_and(|s| s.contains(&name))
}

/// PID semua overlay yang masih hidup (file pid basi dihapus).
pub fn overlay_pids() -> Vec<u32> {
    let Ok(rd) = fs::read_dir(dir()) else {
        return Vec::new();
    };
    let prefix = format!("{NAME}-");
    let mut v = Vec::new();
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let Some(pid) = name
            .strip_prefix(prefix.as_str())
            .and_then(|n| n.strip_suffix(".pid"))
            .and_then(|n| n.parse::<u32>().ok())
        else {
            continue;
        };
        if alive(pid) {
            v.push(pid);
        } else {
            let _ = fs::remove_file(e.path());
        }
    }
    v
}

/// Cek sekali (tanpa polling): dipanggil saat panel dibuka dan saat tombol ditekan.
pub fn overlay_running() -> bool {
    !overlay_pids().is_empty()
}

pub fn stop_overlay() {
    for pid in overlay_pids() {
        let p = pid.to_string();
        #[cfg(windows)]
        let _ = run(Command::new("taskkill").args(["/PID", p.as_str(), "/F"]));
        #[cfg(not(windows))]
        let _ = run(Command::new("kill").arg(p.as_str()));
        let _ = fs::remove_file(pid_file(pid));
    }
}

/// Jalankan diri sendiri dengan `arg` sebagai proses terlepas (hidup setelah panel ditutup).
pub fn spawn(arg: &str) -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let mut c = Command::new(exe);
    c.arg(arg)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0000_0008 | 0x0000_0200); // DETACHED_PROCESS | NEW_PROCESS_GROUP
    }
    c.spawn().is_ok()
}

/// Prioritas CPU di bawah normal untuk proses latar belakang.
pub fn lower_priority() {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::Threading::{
            GetCurrentProcess, SetPriorityClass, BELOW_NORMAL_PRIORITY_CLASS,
        };
        SetPriorityClass(GetCurrentProcess(), BELOW_NORMAL_PRIORITY_CLASS);
    }
}

/// Daftarkan/hapus jalan otomatis saat login. Perintah: `exe --startup`.
pub fn autostart(enable: bool) {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    #[cfg(windows)]
    {
        let key = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
        if enable {
            let v = format!("\"{}\" --startup", exe.display());
            let _ = run(Command::new("reg").args([
                "add",
                key,
                "/v",
                NAME,
                "/t",
                "REG_SZ",
                "/d",
                v.as_str(),
                "/f",
            ]));
        } else {
            let _ = run(Command::new("reg").args(["delete", key, "/v", NAME, "/f"]));
        }
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let Some(home) = std::env::var_os("HOME") else {
            return;
        };
        let dir = PathBuf::from(home).join(".config/autostart");
        let file = dir.join(format!("{NAME}.desktop"));
        if enable {
            let _ = fs::create_dir_all(&dir);
            let _ = fs::write(
                file,
                format!(
                    "[Desktop Entry]\nType=Application\nName=desktopstickman\nExec=\"{}\" --startup\n",
                    exe.display()
                ),
            );
        } else {
            let _ = fs::remove_file(file);
        }
    }
    #[cfg(not(any(windows, all(unix, not(target_os = "macos")))))]
    let _ = (enable, exe);
}

/// Posisi kursor layar + status klik kanan: `(x, y, ditekan)`.
/// Hanya Windows (overlay click-through, jadi dibaca langsung dari OS).
pub fn cursor() -> Option<(i32, i32, bool)> {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::{
            Foundation::POINT,
            UI::{Input::KeyboardAndMouse::GetAsyncKeyState, WindowsAndMessaging::GetCursorPos},
        };
        let mut p = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut p) == 0 {
            return None;
        }
        Some((p.x, p.y, GetAsyncKeyState(0x02) < 0)) // VK_RBUTTON
    }
    #[cfg(not(windows))]
    None
}

const MOD_CTRL: u8 = 1;
const MOD_ALT: u8 = 2;
const MOD_SHIFT: u8 = 4;

/// Tombol bernama (selain A-Z, 0-9, F1-F12, Num0-9).
const NAMED: &[(&str, u32)] = &[
    ("Space", 0x20),
    ("Tab", 0x09),
    ("Enter", 0x0D),
    ("Backspace", 0x08),
    ("Insert", 0x2D),
    ("Delete", 0x2E),
    ("Home", 0x24),
    ("End", 0x23),
    ("PageUp", 0x21),
    ("PageDown", 0x22),
    ("Up", 0x26),
    ("Down", 0x28),
    ("Left", 0x25),
    ("Right", 0x27),
    (";", 0xBA),
    ("=", 0xBB),
    (",", 0xBC),
    ("-", 0xBD),
    (".", 0xBE),
    ("/", 0xBF),
    ("`", 0xC0),
    ("[", 0xDB),
    ("\\", 0xDC),
    ("]", 0xDD),
    ("'", 0xDE),
];

/// Kode tombol virtual dari nama satu tombol (tanpa modifier).
pub fn vk(name: &str) -> Option<u32> {
    let n = name.trim();
    if let Some(&(_, c)) = NAMED.iter().find(|(s, _)| s.eq_ignore_ascii_case(n)) {
        return Some(c);
    }
    let u = n.to_ascii_uppercase();
    let b = u.as_bytes();
    if b.len() == 1 && b[0].is_ascii_alphanumeric() {
        return Some(b[0] as u32);
    }
    if let Some(d) = u
        .strip_prefix("NUM")
        .and_then(|d| d.parse::<u32>().ok())
        .filter(|d| *d <= 9)
    {
        return Some(0x60 + d);
    }
    u.strip_prefix('F')
        .and_then(|d| d.parse::<u32>().ok())
        .filter(|d| (1..=12).contains(d))
        .map(|d| 0x6F + d)
}

/// Nama tombol dari kode virtual (kebalikan `vk`); None = tidak bisa direkam.
fn name_of(code: u32) -> Option<String> {
    if let Some(&(s, _)) = NAMED.iter().find(|(_, c)| *c == code) {
        return Some(s.to_string());
    }
    match code {
        0x30..=0x39 | 0x41..=0x5A => char::from_u32(code).map(String::from),
        0x60..=0x69 => Some(format!("Num{}", code - 0x60)),
        0x70..=0x7B => Some(format!("F{}", code - 0x6F)),
        _ => None,
    }
}

/// Kombinasi seperti "Alt+C" / "Ctrl+Shift+F5" -> (modifier, kode tombol).
pub fn parse(s: &str) -> Option<(u8, u32)> {
    let (mut mods, mut key) = (0u8, None);
    for part in s.split('+') {
        match part.trim().to_ascii_uppercase().as_str() {
            "CTRL" => mods |= MOD_CTRL,
            "ALT" => mods |= MOD_ALT,
            "SHIFT" => mods |= MOD_SHIFT,
            _ if key.is_none() => key = Some(vk(part)?),
            _ => return None,
        }
    }
    key.map(|k| (mods, k))
}

fn mods_now() -> u8 {
    let mut m = 0;
    for (vk, bit) in [(0x11, MOD_CTRL), (0x12, MOD_ALT), (0x10, MOD_SHIFT)] {
        if key_down(vk) {
            m |= bit;
        }
    }
    m
}

/// Kombinasi sedang ditekan (modifier harus persis sama, jadi "C" tidak aktif saat Ctrl+C).
pub fn combo_down(name: &str) -> bool {
    parse(name).is_some_and(|(m, k)| key_down(k) && mods_now() == m)
}

fn codes() -> impl Iterator<Item = u32> {
    (0x08..=0xDEu32).filter(|c| name_of(*c).is_some())
}

/// Tombol (yang bisa direkam) yang sedang ditahan sekarang.
pub fn held() -> Vec<u32> {
    codes().filter(|c| key_down(*c)).collect()
}

/// Perekam tombol: kembalikan kombinasi (mis. "Alt+C") saat ada tombol baru ditekan.
/// `base` = tombol yang sudah ditahan sebelumnya (diabaikan sampai dilepas).
pub fn capture(base: &mut Vec<u32>) -> Option<String> {
    base.retain(|c| key_down(*c));
    let code = codes().find(|c| !base.contains(c) && key_down(*c))?;
    let mods = mods_now();
    let mut s = String::new();
    for (m, n) in [(MOD_CTRL, "Ctrl"), (MOD_ALT, "Alt"), (MOD_SHIFT, "Shift")] {
        if mods & m != 0 {
            s.push_str(n);
            s.push('+');
        }
    }
    s.push_str(&name_of(code)?);
    Some(s)
}

/// Tombol sedang ditekan (global, tanpa fokus). Hanya Windows.
pub fn key_down(vk: u32) -> bool {
    #[cfg(windows)]
    // SAFETY: panggilan Win32 tanpa pointer.
    unsafe {
        windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(vk as i32) < 0
    }
    #[cfg(not(windows))]
    {
        let _ = vk;
        false
    }
}

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

fn pid_path() -> PathBuf {
    dir().join("desktopstickman.pid")
}

pub fn write_pid() {
    let _ = fs::write(pid_path(), std::process::id().to_string());
}

pub fn clear_pid() {
    let _ = fs::remove_file(pid_path());
}

fn read_pid() -> Option<u32> {
    fs::read_to_string(pid_path()).ok()?.trim().parse().ok()
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

/// Cek sekali (tanpa polling): dipanggil saat panel dibuka dan saat tombol ditekan.
pub fn overlay_running() -> bool {
    read_pid().is_some_and(alive)
}

pub fn stop_overlay() {
    if let Some(p) = read_pid().filter(|p| alive(*p)) {
        let p = p.to_string();
        #[cfg(windows)]
        let _ = run(Command::new("taskkill").args(["/PID", p.as_str(), "/F"]));
        #[cfg(not(windows))]
        let _ = run(Command::new("kill").arg(p.as_str()));
    }
    clear_pid();
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

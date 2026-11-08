//! Ikon system tray (Windows). Panel di-minimize -> disembunyikan ke tray; klik kiri = buka,
//! klik kanan = menu. Non-Windows: tidak ada tray (`install` mengembalikan false).
#[derive(Clone, Copy)]
#[cfg_attr(not(windows), allow(dead_code))]
pub enum Ev {
    Show,
    Quit,
}

#[cfg(not(windows))]
pub fn install() -> Option<std::sync::mpsc::Receiver<Ev>> {
    None
}

#[cfg(not(windows))]
pub fn remove() {}

#[cfg(windows)]
pub use win::{install, remove};

#[cfg(windows)]
mod win {
    use super::Ev;
    use std::{
        mem,
        ptr::null,
        sync::{
            atomic::{AtomicIsize, Ordering},
            mpsc::{channel, Receiver, Sender},
            Mutex,
        },
    };
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Shell::{Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE},
            WindowsAndMessaging::{
                AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
                DispatchMessageW, GetCursorPos, GetMessageW, LoadIconW, RegisterClassW,
                SetForegroundWindow, TrackPopupMenu, TranslateMessage, HWND_MESSAGE,
                IDI_APPLICATION, MF_STRING, MSG, TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_LBUTTONUP,
                WM_RBUTTONUP, WNDCLASSW,
            },
        },
    };

    const WM_TRAY: u32 = 0x8001;
    static TX: Mutex<Option<Sender<Ev>>> = Mutex::new(None);
    static HWND_TRAY: AtomicIsize = AtomicIsize::new(0);

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    fn send(e: Ev) {
        if let Ok(g) = TX.lock() {
            if let Some(tx) = g.as_ref() {
                let _ = tx.send(e);
            }
        }
    }

    fn nid(hwnd: HWND) -> windows_sys::Win32::UI::Shell::NOTIFYICONDATAW {
        // SAFETY: struct POD, nol valid untuk semua field.
        let mut n: windows_sys::Win32::UI::Shell::NOTIFYICONDATAW = unsafe { mem::zeroed() };
        n.cbSize = mem::size_of_val(&n) as u32;
        n.hWnd = hwnd;
        n.uID = 1;
        n
    }

    unsafe extern "system" fn proc(h: HWND, m: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        if m == WM_TRAY {
            match (lp as u32) & 0xFFFF {
                WM_LBUTTONUP => send(Ev::Show),
                WM_RBUTTONUP => menu(h),
                _ => {}
            }
            return 0;
        }
        DefWindowProcW(h, m, wp, lp)
    }

    unsafe fn menu(h: HWND) {
        let m = CreatePopupMenu();
        AppendMenuW(m, MF_STRING, 1, wide("Buka panel").as_ptr());
        AppendMenuW(m, MF_STRING, 2, wide("Keluar").as_ptr());
        let mut p: POINT = mem::zeroed();
        GetCursorPos(&mut p);
        SetForegroundWindow(h);
        let id = TrackPopupMenu(m, TPM_RETURNCMD | TPM_RIGHTBUTTON, p.x, p.y, 0, h, null());
        DestroyMenu(m);
        match id {
            1 => send(Ev::Show),
            2 => send(Ev::Quit),
            _ => {}
        }
    }

    /// Buat ikon tray di thread sendiri; event diterima lewat channel (poll dari thread UI).
    pub fn install() -> Option<Receiver<Ev>> {
        let (tx, rx) = channel();
        *TX.lock().ok()? = Some(tx);
        let (ok_tx, ok_rx) = channel::<bool>();
        std::thread::spawn(move || unsafe {
            let class = wide("DesktopStickmanTray");
            let wc = WNDCLASSW {
                lpfnWndProc: Some(proc),
                hInstance: GetModuleHandleW(null()),
                lpszClassName: class.as_ptr(),
                ..mem::zeroed()
            };
            RegisterClassW(&wc);
            let hwnd = CreateWindowExW(
                0,
                class.as_ptr(),
                class.as_ptr(),
                0,
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                0,
                wc.hInstance,
                null(),
            );
            if hwnd == 0 {
                let _ = ok_tx.send(false);
                return;
            }
            let mut n = nid(hwnd);
            n.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
            n.uCallbackMessage = WM_TRAY;
            n.hIcon = LoadIconW(0, IDI_APPLICATION);
            for (d, s) in n.szTip.iter_mut().zip(wide("desktopstickman").iter()) {
                *d = *s;
            }
            let added = Shell_NotifyIconW(NIM_ADD, &n) != 0;
            if added {
                HWND_TRAY.store(hwnd, Ordering::SeqCst);
            }
            let _ = ok_tx.send(added);
            let mut msg: MSG = mem::zeroed();
            while GetMessageW(&mut msg, 0, 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        });
        ok_rx.recv().unwrap_or(false).then_some(rx)
    }

    pub fn remove() {
        let h = HWND_TRAY.swap(0, Ordering::SeqCst);
        if h != 0 {
            // SAFETY: hanya menghapus ikon milik jendela tray ini.
            unsafe { Shell_NotifyIconW(NIM_DELETE, &nid(h)) };
        }
    }
}

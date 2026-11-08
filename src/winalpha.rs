//! Alpha per-piksel di Windows via layered window + UpdateLayeredWindow (premultiplied BGRA).
use std::{ffi::c_void, mem, ptr::null_mut};
use windows_sys::Win32::{
    Foundation::{HWND, POINT, SIZE},
    Graphics::Gdi::{
        CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject, AC_SRC_ALPHA,
        AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS, HBITMAP,
        HDC,
    },
    UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, UpdateLayeredWindow, GWL_EXSTYLE, ULW_ALPHA,
        WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT,
    },
};

pub struct Layered {
    hwnd: HWND,
    dc: HDC,
    bmp: HBITMAP,
    bits: *mut u32,
    w: i32,
    h: i32,
}

impl Layered {
    pub fn new(hwnd: isize, w: i32, h: i32) -> Option<Self> {
        unsafe {
            let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            let add = WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE;
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex | add as isize);
            let dc = CreateCompatibleDC(0);
            let mut bi: BITMAPINFO = mem::zeroed();
            bi.bmiHeader.biSize = mem::size_of::<BITMAPINFOHEADER>() as u32;
            bi.bmiHeader.biWidth = w;
            bi.bmiHeader.biHeight = -h; // top-down
            bi.bmiHeader.biPlanes = 1;
            bi.bmiHeader.biBitCount = 32;
            bi.bmiHeader.biCompression = BI_RGB;
            let mut bits: *mut c_void = null_mut();
            let bmp = CreateDIBSection(dc, &bi, DIB_RGB_COLORS, &mut bits, 0, 0);
            if bmp == 0 || bits.is_null() {
                DeleteDC(dc);
                return None;
            }
            SelectObject(dc, bmp);
            Some(Self {
                hwnd,
                dc,
                bmp,
                bits: bits as *mut u32,
                w,
                h,
            })
        }
    }

    /// `px` = w*h piksel premultiplied ARGB; `(x, y)` = posisi jendela di layar.
    pub fn present(&self, px: &[u32], x: i32, y: i32) {
        if px.len() < (self.w * self.h) as usize {
            return;
        }
        unsafe {
            std::ptr::copy_nonoverlapping(px.as_ptr(), self.bits, (self.w * self.h) as usize);
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            let pos = POINT { x, y };
            let size = SIZE {
                cx: self.w,
                cy: self.h,
            };
            let src = POINT { x: 0, y: 0 };
            UpdateLayeredWindow(
                self.hwnd, 0, &pos, &size, self.dc, &src, 0, &blend, ULW_ALPHA,
            );
        }
    }
}

impl Drop for Layered {
    fn drop(&mut self) {
        unsafe {
            DeleteObject(self.bmp);
            DeleteDC(self.dc);
        }
    }
}

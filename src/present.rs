//! Presentasi buffer ke jendela overlay: softbuffer (Linux/macOS) atau layered window (Windows).
use crate::stick::Canvas;
use std::sync::Arc;
use winit::{dpi::PhysicalPosition, window::Window};

pub struct Presenter {
    win: Arc<Window>,
    w: usize,
    h: usize,
    buf: Vec<u32>,
    #[cfg(windows)]
    lay: Option<crate::winalpha::Layered>,
    #[cfg(not(windows))]
    surf: softbuffer::Surface<Arc<Window>, Arc<Window>>,
    #[cfg(not(windows))]
    _ctx: softbuffer::Context<Arc<Window>>,
}

impl Presenter {
    #[cfg(windows)]
    pub fn new(win: &Arc<Window>, w: u32, h: u32) -> Self {
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
        let lay = match win.window_handle().map(|h| h.as_raw()) {
            Ok(RawWindowHandle::Win32(hd)) => {
                crate::winalpha::Layered::new(hd.hwnd.get(), w as i32, h as i32)
            }
            _ => None,
        };
        Self {
            win: win.clone(),
            w: w as usize,
            h: h as usize,
            buf: vec![0; (w * h) as usize],
            lay,
        }
    }

    #[cfg(not(windows))]
    pub fn new(win: &Arc<Window>, w: u32, h: u32) -> Self {
        let ctx = softbuffer::Context::new(win.clone()).expect("ctx");
        let mut surf = softbuffer::Surface::new(&ctx, win.clone()).expect("surface");
        let _ = surf.resize(
            std::num::NonZeroU32::new(w).unwrap_or(std::num::NonZeroU32::MIN),
            std::num::NonZeroU32::new(h).unwrap_or(std::num::NonZeroU32::MIN),
        );
        Self {
            win: win.clone(),
            w: w as usize,
            h: h as usize,
            buf: vec![0; (w * h) as usize],
            surf,
            _ctx: ctx,
        }
    }

    fn fill(dst: &mut [u32], c: &Canvas, cell: usize, w: usize) {
        for (y, row) in dst.chunks_mut(w).enumerate() {
            for (x, px) in row.iter_mut().enumerate() {
                *px = c.px.get((y / cell) * c.w + x / cell).copied().unwrap_or(0);
            }
        }
    }

    pub fn present(&mut self, c: &Canvas, cell: usize, pos: PhysicalPosition<i32>) {
        #[cfg(windows)]
        {
            Self::fill(&mut self.buf, c, cell, self.w);
            if let Some(l) = &self.lay {
                l.present(&self.buf, pos.x, pos.y);
            }
            let _ = (&self.win, self.h);
        }
        #[cfg(not(windows))]
        {
            self.win.set_outer_position(pos);
            let _ = (&self.buf, self.h);
            if let Ok(mut b) = self.surf.buffer_mut() {
                Self::fill(&mut b, c, cell, self.w);
                let _ = b.present();
            }
        }
    }
}

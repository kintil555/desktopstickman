mod config;
mod fx;
mod stick;
mod veh;

use config::Config;
use std::{
    num::NonZeroU32,
    sync::Arc,
    time::{Duration, Instant},
};
use stick::{Canvas, Fly, Rng, Stick, V};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{WindowBuilder, WindowLevel},
};

const WIN_W: u32 = 320;
const WIN_H: u32 = 200;
/// Posisi telapak kaki di dalam jendela (px dari atas).
const FEET_Y: f32 = 165.0;

fn seed() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |d| d.subsec_nanos())
}

fn render(s: &Stick, cfg: &Config) -> Canvas {
    let mut c = Canvas::new(WIN_W, WIN_H, cfg.cell);
    let floor_y = FEET_Y + s.alt;
    let ox = WIN_W as f32 / 2.0;
    if cfg.dust {
        for d in &s.fx {
            fx::draw(&mut c, d, s.x, ox, floor_y);
        }
    }
    if s.fly == Fly::No {
        let o = V {
            x: ox,
            y: FEET_Y - s.oy,
        };
        c.draw(&s.pose(), o, cfg);
    } else {
        let o = V {
            x: ox,
            y: FEET_Y + s.bob(),
        };
        veh::draw(&mut c, s, o, cfg, floor_y);
    }
    c
}

/// Tes headless: tulis strip frame ke PPM (latar abu-abu muda).
fn dump(path: &str, fly: bool) {
    let cfg = Config::default();
    let mut rng = Rng::new(7);
    let mut s = Stick::new(600.0);
    // lewati fase idle: tunggu sampai berlari
    for _ in 0..20000 {
        let ready = if fly {
            s.fly == Fly::Cruise
        } else {
            s.vx.abs() > 300.0
        };
        if ready {
            break;
        }
        if fly {
            s.force_fly();
        }
        s.update(1.0 / 60.0, &cfg, &mut rng, 1200.0, 300.0);
    }
    let (frames, step) = (12usize, 1.0 / 30.0);
    let (w, h) = (WIN_W as usize, WIN_H as usize);
    let mut img = vec![200u8; w * frames * h * 3];
    for f in 0..frames {
        for _ in 0..2 {
            s.update(step, &cfg, &mut rng, 1200.0, 300.0);
        }
        let c = render(&s, &cfg);
        for y in 0..h {
            for x in 0..w {
                let p = c.px[(y / cfg.cell as usize) * c.w + x / cfg.cell as usize];
                if p >> 24 != 0 {
                    let o = (y * w * frames + f * w + x) * 3;
                    img[o] = (p >> 16) as u8;
                    img[o + 1] = (p >> 8) as u8;
                    img[o + 2] = p as u8;
                }
            }
        }
    }
    let mut out = format!("P6\n{} {}\n255\n", w * frames, h).into_bytes();
    out.extend(img);
    std::fs::write(path, out).expect("tulis dump");
}

fn overlay() {
    let mut cfg = Config::load();
    let mut cfg_mtime = Config::mtime();
    let el = EventLoop::new().expect("event loop");
    let win = Arc::new(
        WindowBuilder::new()
            .with_title("desktopstickman")
            .with_inner_size(PhysicalSize::new(WIN_W, WIN_H))
            .with_decorations(false)
            .with_transparent(true)
            .with_resizable(false)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .build(&el)
            .expect("window"),
    );
    let _ = win.set_cursor_hittest(false);
    let ctx = softbuffer::Context::new(win.clone()).expect("ctx");
    let mut surf = softbuffer::Surface::new(&ctx, win.clone()).expect("surface");
    let _ = surf.resize(
        NonZeroU32::new(WIN_W).unwrap_or(NonZeroU32::MIN),
        NonZeroU32::new(WIN_H).unwrap_or(NonZeroU32::MIN),
    );

    let (mon_pos, mon_size) = win
        .primary_monitor()
        .or_else(|| win.current_monitor())
        .map_or(
            (PhysicalPosition::new(0, 0), PhysicalSize::new(1920, 1080)),
            |m| (m.position(), m.size()),
        );
    let max_x = mon_size.width as f32;
    let max_alt = (mon_size.height as i32 - cfg.floor_offset - 250).max(100) as f32;
    let mut rng = Rng::new(seed());
    let mut s = Stick::new(max_x / 2.0);
    let frame = Duration::from_millis(16);
    let (mut next, mut last, mut chk) = (Instant::now(), Instant::now(), Instant::now());

    let _ = el.run(move |event, elwt| match event {
        Event::AboutToWait => {
            let now = Instant::now();
            if now >= next {
                next = now + frame;
                win.request_redraw();
            }
            elwt.set_control_flow(ControlFlow::WaitUntil(next));
        }
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::CloseRequested => elwt.exit(),
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = now.duration_since(last).as_secs_f32().min(0.05);
                last = now;
                if now.duration_since(chk) > Duration::from_secs(1) {
                    chk = now;
                    let mt = Config::mtime();
                    if mt != cfg_mtime {
                        cfg_mtime = mt;
                        cfg = Config::load();
                    }
                }
                s.update(dt, &cfg, &mut rng, max_x, max_alt);
                let floor_y = mon_pos.y + mon_size.height as i32 - cfg.floor_offset;
                win.set_outer_position(PhysicalPosition::new(
                    mon_pos.x + s.x as i32 - WIN_W as i32 / 2,
                    floor_y - FEET_Y as i32 - s.alt as i32,
                ));
                let c = render(&s, &cfg);
                if let Ok(mut buf) = surf.buffer_mut() {
                    let cell = cfg.cell as usize;
                    for (y, row) in buf.chunks_mut(WIN_W as usize).enumerate() {
                        for (x, px) in row.iter_mut().enumerate() {
                            *px = c.px.get((y / cell) * c.w + x / cell).copied().unwrap_or(0);
                        }
                    }
                    let _ = buf.present();
                }
            }
            _ => {}
        },
        _ => {}
    });
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("--dump") => dump(
            args.get(2).map_or("dump.ppm", String::as_str),
            args.get(3).is_some_and(|a| a == "fly"),
        ),
        // TODO: tanpa argumen = panel Slint (lihat NOTES.md); sementara jalankan overlay.
        _ => overlay(),
    }
}

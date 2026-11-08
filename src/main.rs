#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod config;
mod fx;
mod hang;
mod panel;
mod present;
mod shadow;
mod skin;
mod stick;
mod sys;
mod tray;
mod veh;
#[cfg(windows)]
mod winalpha;

use config::Config;
use hang::{Hang, Phase, HAND_Y};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use stick::{Canvas, Fly, Rng, Stick, GRAV, V};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{WindowBuilder, WindowLevel},
};

const WIN_W: u32 = 320;
const WIN_H: u32 = 260;
/// Posisi telapak kaki di dalam jendela (px dari atas).
const FEET_Y: f32 = 225.0;

fn seed() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |d| {
            d.subsec_nanos() ^ std::process::id().wrapping_mul(2_654_435_761)
        })
}

fn render(s: &Stick, cfg: &Config, sheet: Option<&skin::Sheet>) -> Canvas {
    let mut c = Canvas::new(WIN_W, WIN_H, cfg.cell);
    let floor_y = FEET_Y + s.alt;
    let ox = WIN_W as f32 / 2.0;
    if cfg.dust {
        fx::draw_all(&mut c, &s.fx, s.x, ox, floor_y);
        fx::draw_air(&mut c, &s.smoke, &s.sparks, s.x, ox, floor_y);
    }
    if s.fly == Fly::No {
        let o = V {
            x: ox,
            y: FEET_Y - s.oy,
        };
        match sheet {
            Some(sh) => skin::draw_player(&mut c, sh, cfg, s, o, false, None),
            None => c.draw(&s.pose(), o, cfg),
        }
    } else {
        let o = V {
            x: ox,
            y: FEET_Y + s.bob(),
        };
        veh::draw(&mut c, s, o, cfg, floor_y, sheet);
    }
    shadow::apply(&mut c, cfg);
    c
}

/// Stickman bergantung di kursor: tangan di (WIN_W/2, HAND_Y) pada kanvas.
fn render_hang(h: &Hang, s: &Stick, cfg: &Config, sheet: Option<&skin::Sheet>) -> Canvas {
    let mut c = Canvas::new(WIN_W, WIN_H, cfg.cell);
    let o = V {
        x: WIN_W as f32 / 2.0,
        y: HAND_Y,
    };
    match sheet {
        Some(sh) => {
            let mut st = Stick::new(0.0);
            st.face = h.face;
            st.t = s.t;
            skin::draw_player(&mut c, sh, cfg, &st, o, false, Some((h.angle(), o)));
        }
        None => c.draw(&h.pose(), o, cfg),
    }
    shadow::apply(&mut c, cfg);
    c
}

fn load_sheet(cfg: &Config) -> Option<skin::Sheet> {
    if cfg.skin.enabled {
        skin::load(&cfg.skin.path)
    } else {
        None
    }
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
        let c = render(&s, &cfg, None);
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

fn overlay(id: u32) {
    sys::lower_priority();
    sys::write_pid(id);
    let mut cfg = Config::load();
    if !cfg.apply_inst(id) {
        sys::clear_pid();
        return;
    }
    let mut sheet = load_sheet(&cfg);
    let mut cfg_mtime = Config::mtime();
    let el = EventLoop::new().expect("event loop");
    let wb = WindowBuilder::new()
        .with_title("desktopstickman")
        .with_inner_size(PhysicalSize::new(WIN_W, WIN_H))
        .with_decorations(false)
        .with_transparent(cfg!(not(windows)))
        .with_resizable(false)
        .with_window_level(WindowLevel::AlwaysOnTop);
    // tanpa tombol di taskbar sejak dibuat (WS_EX_TOOLWINDOW setelah tampil tidak cukup)
    #[cfg(windows)]
    let wb = winit::platform::windows::WindowBuilderExtWindows::with_skip_taskbar(wb, true);
    let win = Arc::new(wb.build(&el).expect("window"));
    let _ = win.set_cursor_hittest(false);
    let mut out = present::Presenter::new(&win, WIN_W, WIN_H);

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
    // banyak stickman = banyak proses; posisi awal acak supaya tidak bertumpuk
    let mut s = Stick::new(max_x * (0.1 + 0.8 * rng.f()));
    let (mut next, mut last, mut chk) = (Instant::now(), Instant::now(), Instant::now());
    // hemat: lewati present bila gambar & posisi sama; idle di lantai cukup ~30 fps
    let mut prev: (Vec<u32>, PhysicalPosition<i32>) = (Vec::new(), PhysicalPosition::new(0, 0));
    let mut frame = Duration::from_millis(16);
    let mut hang: Option<Hang> = None;
    let mut rb_prev = false;
    // true = ikut kursor lewat tombol pintas (toggle); false = lewat tahan klik kanan
    let mut hang_key = false;
    let mut key_prev = [false; 4];
    let mut shown = true;

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
                        // dihapus dari daftar (di panel) = stickman ini keluar
                        if !cfg.apply_inst(id) {
                            elwt.exit();
                            return;
                        }
                        sheet = load_sheet(&cfg);
                        prev.0.clear();
                    }
                }
                // tombol pintas global: [gantung, lompat, kendaraan, sembunyikan]
                let kd = [
                    &cfg.keys.hang,
                    &cfg.keys.jump,
                    &cfg.keys.fly,
                    &cfg.keys.hide,
                ]
                .map(|k| sys::combo_down(k.as_str()));
                let edge: [bool; 4] = std::array::from_fn(|i| kd[i] && !key_prev[i]);
                key_prev = kd;
                if edge[1] {
                    s.jump_now();
                }
                if edge[2] {
                    s.force_fly();
                }
                if edge[3] {
                    shown = !shown;
                    win.set_visible(shown);
                }
                // ikut kursor: tahan klik kanan di stickman (lepas = turun), atau tombol pintas
                // di atas stickman (tekan lagi = lepas)
                if let Some((cx, cy, down)) = cfg.hang.then(sys::cursor).flatten() {
                    let press = down && !rb_prev;
                    rb_prev = down;
                    let cur = V {
                        x: cx as f32,
                        y: cy as f32,
                    };
                    if hang.is_none() {
                        let feet = V {
                            x: (prev.1.x + WIN_W as i32 / 2) as f32,
                            y: (prev.1.y + FEET_Y as i32) as f32,
                        };
                        let hit = (cur.x - feet.x).abs() <= 30.0
                            && cur.y >= feet.y - 100.0
                            && cur.y <= feet.y + 8.0;
                        if (press || edge[0]) && hit && s.fly == Fly::No {
                            hang = Some(Hang::grab(cur, feet, s.face));
                            hang_key = !press;
                        }
                    } else if let Some(h) = hang.as_mut() {
                        let drop = if hang_key {
                            edge[0] && h.age > 0.3
                        } else {
                            !down
                        };
                        if drop && !h.falling() {
                            h.release();
                            s.begin_fall();
                        }
                    }
                    if let Some(h) = hang.as_mut().filter(|h| !h.falling()) {
                        h.update(dt, cur);
                    }
                }
                let floor_y = mon_pos.y + mon_size.height as i32 - cfg.floor_offset;
                let mut landed = None;
                if let Some(h) = hang.as_mut().filter(|h| h.falling()) {
                    if h.fall(dt, floor_y as f32) {
                        landed = Some((h.feet.x, h.vel));
                    }
                }
                if let Some((x, vel)) = landed {
                    hang = None;
                    let drop_h = vel.y * vel.y / (2.0 * GRAV);
                    let x = (x - mon_pos.x as f32).clamp(0.0, max_x);
                    s.land(x, vel.x * 0.5, drop_h, &mut rng);
                }
                let (pos, img) = match hang.as_ref() {
                    Some(h) if h.phase != Phase::Fall => (
                        PhysicalPosition::new(
                            h.hand.x as i32 - WIN_W as i32 / 2,
                            h.hand.y as i32 - HAND_Y as i32,
                        ),
                        render_hang(h, &s, &cfg, sheet.as_ref()),
                    ),
                    Some(h) => {
                        // jatuh bebas: pose udara, jendela mengikuti telapak kaki
                        s.fall_tick(h.vel.x, dt);
                        (
                            PhysicalPosition::new(
                                h.feet.x as i32 - WIN_W as i32 / 2,
                                h.feet.y as i32 - FEET_Y as i32,
                            ),
                            render(&s, &cfg, sheet.as_ref()),
                        )
                    }
                    None => {
                        s.update(dt, &cfg, &mut rng, max_x, max_alt);
                        let pos = PhysicalPosition::new(
                            mon_pos.x + s.x as i32 - WIN_W as i32 / 2,
                            floor_y - FEET_Y as i32 - s.alt as i32,
                        );
                        (pos, render(&s, &cfg, sheet.as_ref()))
                    }
                };
                let idle = hang.is_none()
                    && s.fly == Fly::No
                    && s.on()
                    && s.vx.abs() < 1.0
                    && s.fx.is_empty();
                frame = Duration::from_millis(if idle { 33 } else { 16 });
                if prev.1 != pos || prev.0 != img.px {
                    out.present(&img, cfg.cell as usize, pos);
                    prev = (img.px, pos);
                }
            }
            _ => {}
        },
        _ => {}
    });
    sys::clear_pid();
}

/// Dipanggil autostart: tidur (tanpa CPU) `startup_delay` detik, lalu jalankan overlay.
fn startup() {
    sys::lower_priority();
    std::thread::sleep(Duration::from_secs(Config::load().startup_delay as u64));
    if sys::overlay_running() {
        return;
    }
    let ids: Vec<u32> = Config::load().stickmen.iter().map(|i| i.id).collect();
    if ids.is_empty() {
        overlay(0);
    }
    for id in ids {
        sys::spawn(&["--overlay", id.to_string().as_str()]);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("--dump") => dump(
            args.get(2).map_or("dump.ppm", String::as_str),
            args.get(3).is_some_and(|a| a == "fly"),
        ),
        Some("--overlay") => overlay(args.get(2).and_then(|a| a.parse().ok()).unwrap_or(0)),
        Some("--startup") => startup(),
        _ => panel::run(),
    }
}

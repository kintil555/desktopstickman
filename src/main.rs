#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod config;
mod fx;
mod hang;
mod held;
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

use config::{Config, Inst};
use hang::{Hang, Phase, HAND_Y};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use stick::{Canvas, Fly, Rng, Stick, GRAV, V};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
    window::{Window, WindowBuilder, WindowLevel},
};

const WIN_W: u32 = 320;
const WIN_H: u32 = 260;
/// Posisi telapak kaki di dalam jendela (px dari atas).
const FEET_Y: f32 = 225.0;

fn seed() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |d| d.subsec_nanos())
}

fn render(s: &Stick, cfg: &Config, sheet: Option<&skin::Sheet>, hi: Option<held::Draw>) -> Canvas {
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
        let pose = s.pose();
        match sheet {
            Some(sh) => skin::draw_player(&mut c, sh, cfg, s, o, false, None),
            None => c.draw(&pose, o, cfg),
        }
        if let Some(d) = hi {
            held::draw(&mut c, d, &pose, o, s.face);
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
fn render_hang(
    h: &Hang,
    s: &Stick,
    cfg: &Config,
    sheet: Option<&skin::Sheet>,
    hi: Option<held::Draw>,
) -> Canvas {
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
    if let Some(d) = hi {
        held::draw(&mut c, d, &h.pose(), o, h.face);
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
        let c = render(&s, &cfg, None, None);
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

/// Keadaan bersama semua stickman (satu proses, satu event loop, satu jendela per stickman).
struct Env {
    mon_pos: PhysicalPosition<i32>,
    mon_size: PhysicalSize<u32>,
    sheet: Option<skin::Sheet>,
    /// Gambar item (PNG) per path.
    arts: held::Arts,
    /// Tepi tombol pintas pada tick ini: [ikut, lompat, kendaraan, sembunyikan, ganti item].
    edge: [bool; 5],
    shown: bool,
    /// Sudah ada stickman yang dibawa kursor (hanya satu boleh sekaligus).
    taken: bool,
    /// Ada stickman yang bergerak pada tick ini (menentukan fps).
    busy: bool,
}

impl Env {
    fn new(t: &EventLoopWindowTarget<()>, sheet: Option<skin::Sheet>) -> Self {
        let (mon_pos, mon_size) = match t.primary_monitor() {
            Some(m) => (m.position(), m.size()),
            None => (PhysicalPosition::new(0, 0), PhysicalSize::new(1920, 1080)),
        };
        Self {
            mon_pos,
            mon_size,
            sheet,
            arts: held::Arts::new(),
            edge: [false; 5],
            shown: true,
            taken: false,
            busy: true,
        }
    }
}

/// Satu stickman: jendela + state sendiri.
struct Agent {
    id: u32,
    win: Arc<Window>,
    out: present::Presenter,
    /// Config global yang sudah ditimpa pengaturan stickman ini.
    cfg: Config,
    /// Item yang sedang dipegang (id; 0 = kosong); bisa diganti dengan tombol pintas.
    held: u32,
    s: Stick,
    rng: Rng,
    last: Instant,
    /// Hemat: lewati present bila gambar & posisi sama.
    prev: (Vec<u32>, PhysicalPosition<i32>),
    hang: Option<Hang>,
    rb_prev: bool,
    /// true = ikut kursor lewat tombol pintas (toggle); false = lewat tahan klik kanan.
    hang_key: bool,
}

impl Agent {
    fn new(t: &EventLoopWindowTarget<()>, id: u32, cfg: Config, env: &Env) -> Self {
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
        let win = Arc::new(wb.build(t).expect("window"));
        let _ = win.set_cursor_hittest(false);
        win.set_visible(env.shown);
        let out = present::Presenter::new(&win, WIN_W, WIN_H);
        let held = cfg.held;
        let mut rng = Rng::new(seed() ^ id.wrapping_mul(0x9E37_79B1));
        // posisi awal acak supaya stickman tidak bertumpuk
        let s = Stick::new(env.mon_size.width as f32 * (0.1 + 0.8 * rng.f()));
        Self {
            id,
            win,
            out,
            cfg,
            held,
            s,
            rng,
            last: Instant::now(),
            prev: (Vec::new(), PhysicalPosition::new(0, 0)),
            hang: None,
            rb_prev: false,
            hang_key: false,
        }
    }

    /// Satu frame: ikut kursor, gerak, gambar, tampilkan.
    fn step(&mut self, env: &mut Env, now: Instant) {
        let dt = now.duration_since(self.last).as_secs_f32().min(0.05);
        self.last = now;
        let (mon_pos, mon_size) = (env.mon_pos, env.mon_size);
        let max_x = mon_size.width as f32;
        let max_alt = (mon_size.height as i32 - self.cfg.floor_offset - 250).max(100) as f32;
        // ikut kursor: tahan klik kanan di stickman (lepas = turun), atau tombol pintas
        // di atas stickman (tekan lagi = lepas)
        if let Some((cx, cy, down)) = self.cfg.hang.then(sys::cursor).flatten() {
            let press = down && !self.rb_prev;
            self.rb_prev = down;
            let cur = V {
                x: cx as f32,
                y: cy as f32,
            };
            if self.hang.is_none() {
                let feet = V {
                    x: (self.prev.1.x + WIN_W as i32 / 2) as f32,
                    y: (self.prev.1.y + FEET_Y as i32) as f32,
                };
                let hit = (cur.x - feet.x).abs() <= 30.0
                    && cur.y >= feet.y - 100.0
                    && cur.y <= feet.y + 8.0;
                if (press || env.edge[0]) && hit && self.s.fly == Fly::No && !env.taken {
                    self.hang = Some(Hang::grab(cur, feet, self.s.face));
                    self.hang_key = !press;
                    env.taken = true;
                }
            } else if let Some(h) = self.hang.as_mut() {
                let drop = if self.hang_key {
                    env.edge[0] && h.age > 0.3
                } else {
                    !down
                };
                if drop && !h.falling() {
                    h.release();
                    self.s.begin_fall();
                }
            }
            if let Some(h) = self.hang.as_mut().filter(|h| !h.falling()) {
                h.update(dt, cur);
            }
        }
        let floor_y = mon_pos.y + mon_size.height as i32 - self.cfg.floor_offset;
        let mut landed = None;
        if let Some(h) = self.hang.as_mut().filter(|h| h.falling()) {
            if h.fall(dt, floor_y as f32) {
                landed = Some((h.feet.x, h.vel));
            }
        }
        if let Some((x, vel)) = landed {
            self.hang = None;
            let drop_h = vel.y * vel.y / (2.0 * GRAV);
            let x = (x - mon_pos.x as f32).clamp(0.0, max_x);
            self.s.land(x, vel.x * 0.5, drop_h, &mut self.rng);
        }
        let sheet = env.sheet.as_ref();
        let hi = held::pick(&self.cfg.items, self.held, &env.arts);
        let (pos, img) = match self.hang.as_ref() {
            Some(h) if h.phase != Phase::Fall => (
                PhysicalPosition::new(
                    h.hand.x as i32 - WIN_W as i32 / 2,
                    h.hand.y as i32 - HAND_Y as i32,
                ),
                render_hang(h, &self.s, &self.cfg, sheet, hi),
            ),
            Some(h) => {
                // jatuh bebas: pose udara, jendela mengikuti telapak kaki
                self.s.fall_tick(h.vel.x, dt);
                (
                    PhysicalPosition::new(
                        h.feet.x as i32 - WIN_W as i32 / 2,
                        h.feet.y as i32 - FEET_Y as i32,
                    ),
                    render(&self.s, &self.cfg, sheet, hi),
                )
            }
            None => {
                self.s.update(dt, &self.cfg, &mut self.rng, max_x, max_alt);
                let pos = PhysicalPosition::new(
                    mon_pos.x + self.s.x as i32 - WIN_W as i32 / 2,
                    floor_y - FEET_Y as i32 - self.s.alt as i32,
                );
                (pos, render(&self.s, &self.cfg, sheet, hi))
            }
        };
        let idle = self.hang.is_none()
            && self.s.fly == Fly::No
            && self.s.on()
            && self.s.vx.abs() < 1.0
            && self.s.fx.is_empty();
        env.busy |= !idle;
        if self.prev.1 != pos || self.prev.0 != img.px {
            self.out.present(&img, self.cfg.cell as usize, pos);
            self.prev = (img.px, pos);
        }
    }
}

/// Samakan daftar stickman hidup dengan config: yang dihapus ditutup, yang baru dibuat.
fn sync_agents(agents: &mut Vec<Agent>, t: &EventLoopWindowTarget<()>, cfg: &Config, env: &Env) {
    agents.retain_mut(|a| {
        let mut c = cfg.clone();
        let keep = c.apply_inst(a.id);
        if keep {
            if c.held != a.cfg.held {
                a.held = c.held;
            }
            a.cfg = c;
            a.prev.0.clear();
        }
        keep
    });
    for i in &cfg.stickmen {
        if !agents.iter().any(|a| a.id == i.id) {
            let mut c = cfg.clone();
            c.apply_inst(i.id);
            agents.push(Agent::new(t, i.id, c, env));
        }
    }
}

/// Satu proses overlay untuk SEMUA stickman (jendela masing-masing, satu event loop).
fn overlay() {
    sys::lower_priority();
    sys::write_pid();
    let mut cfg = Config::load();
    if cfg.stickmen.is_empty() {
        // config lama tanpa daftar: stickman pertama dari pengaturan global
        let i = Inst::from_cfg(1, &cfg);
        cfg.stickmen.push(i);
        cfg.save();
    }
    let mut cfg_mtime = Config::mtime();
    let el = EventLoop::new().expect("event loop");
    let mut env = Env::new(&el, load_sheet(&cfg));
    held::sync(&mut env.arts, &cfg.items);
    let mut agents: Vec<Agent> = Vec::new();
    sync_agents(&mut agents, &el, &cfg, &env);
    let (mut next, mut chk) = (Instant::now(), Instant::now());
    let mut key_prev = [false; 5];

    let _ = el.run(move |event, elwt| match event {
        Event::AboutToWait => {
            let now = Instant::now();
            if now >= next {
                // satu tick bersama: ~60 fps bila ada yang bergerak, ~30 fps bila semua diam
                next = now + Duration::from_millis(if env.busy { 16 } else { 33 });
                env.busy = false;
                if now.duration_since(chk) > Duration::from_millis(500) {
                    chk = now;
                    let mt = Config::mtime();
                    if mt != cfg_mtime {
                        cfg_mtime = mt;
                        cfg = Config::load();
                        env.sheet = load_sheet(&cfg);
                        held::sync(&mut env.arts, &cfg.items);
                        sync_agents(&mut agents, elwt, &cfg, &env);
                        // semua dihapus dari daftar (di panel) = overlay berhenti
                        if agents.is_empty() {
                            elwt.exit();
                        }
                    }
                }
                // tombol pintas global: dibaca sekali per tick untuk semua stickman
                let kd = [
                    &cfg.keys.hang,
                    &cfg.keys.jump,
                    &cfg.keys.fly,
                    &cfg.keys.hide,
                    &cfg.keys.item,
                ]
                .map(|k| sys::combo_down(k.as_str()));
                env.edge = std::array::from_fn(|i| kd[i] && !key_prev[i]);
                key_prev = kd;
                if env.edge[3] {
                    env.shown = !env.shown;
                }
                env.taken = agents
                    .iter()
                    .any(|a| a.hang.as_ref().is_some_and(|h| !h.falling()));
                for a in &mut agents {
                    if env.edge[3] {
                        a.win.set_visible(env.shown);
                    }
                    if env.edge[1] {
                        a.s.jump_now();
                    }
                    if env.edge[2] {
                        a.s.force_fly();
                    }
                    if env.edge[4] {
                        a.held = held::next(&a.cfg.items, a.held);
                    }
                    a.win.request_redraw();
                }
            }
            elwt.set_control_flow(ControlFlow::WaitUntil(next));
        }
        Event::WindowEvent { window_id, event } => match event {
            WindowEvent::CloseRequested => elwt.exit(),
            WindowEvent::RedrawRequested => {
                if let Some(a) = agents.iter_mut().find(|a| a.win.id() == window_id) {
                    a.step(&mut env, Instant::now());
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
    if !sys::overlay_running() {
        overlay();
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("--dump") => dump(
            args.get(2).map_or("dump.ppm", String::as_str),
            args.get(3).is_some_and(|a| a == "fly"),
        ),
        Some("--overlay") => overlay(),
        Some("--startup") => startup(),
        _ => panel::run(),
    }
}

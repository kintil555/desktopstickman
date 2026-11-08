//! Panel pengaturan (Slint). Start = jalankan `--overlay` sebagai proses anak, Stop = kill.
use crate::config::{self, Config};
use slint::ComponentHandle;
use std::{
    cell::RefCell,
    process::{Child, Command},
    rc::Rc,
    time::Duration,
};

mod ui {
    #![allow(deprecated, clippy::all, unused)]
    slint::include_modules!();
}
use ui::Panel;

fn load(ui: &Panel, c: &Config) {
    ui.set_speed(c.speed);
    ui.set_cell(c.cell as f32);
    ui.set_floor_offset(c.floor_offset as f32);
    ui.set_volume(c.volume);
    ui.set_jump(c.jump);
    ui.set_run(c.run);
    ui.set_fly(c.fly);
    ui.set_dust(c.dust);
    ui.set_sound(c.sound);
    ui.set_color_f(config::to_hex(c.color_f).into());
    ui.set_color_b(config::to_hex(c.color_b).into());
}

fn collect(ui: &Panel) -> Config {
    let old = Config::load();
    Config {
        cell: ui.get_cell().round() as u32,
        speed: ui.get_speed(),
        floor_offset: ui.get_floor_offset().round() as i32,
        volume: ui.get_volume(),
        jump: ui.get_jump(),
        run: ui.get_run(),
        fly: ui.get_fly(),
        dust: ui.get_dust(),
        sound: ui.get_sound(),
        color_f: config::parse_hex(&ui.get_color_f()).unwrap_or(old.color_f),
        color_b: config::parse_hex(&ui.get_color_b()).unwrap_or(old.color_b),
    }
}

fn stop(child: &mut Option<Child>) {
    if let Some(mut c) = child.take() {
        let _ = c.kill();
        let _ = c.wait();
    }
}

pub fn run() {
    let ui = Panel::new().expect("panel");
    load(&ui, &Config::load());
    let child: Rc<RefCell<Option<Child>>> = Rc::new(RefCell::new(None));

    ui.on_changed({
        let w = ui.as_weak();
        move || {
            if let Some(ui) = w.upgrade() {
                collect(&ui).save();
            }
        }
    });
    ui.on_toggle({
        let (w, child) = (ui.as_weak(), child.clone());
        move || {
            let Some(ui) = w.upgrade() else { return };
            let mut ch = child.borrow_mut();
            if ch.is_some() {
                stop(&mut ch);
            } else {
                collect(&ui).save();
                *ch = std::env::current_exe()
                    .ok()
                    .and_then(|e| Command::new(e).arg("--overlay").spawn().ok());
            }
            ui.set_running(ch.is_some());
        }
    });
    // deteksi overlay yang berhenti sendiri
    let timer = slint::Timer::default();
    timer.start(slint::TimerMode::Repeated, Duration::from_millis(500), {
        let (w, child) = (ui.as_weak(), child.clone());
        move || {
            let mut ch = child.borrow_mut();
            if let Some(c) = ch.as_mut() {
                if matches!(c.try_wait(), Ok(Some(_))) {
                    *ch = None;
                    if let Some(ui) = w.upgrade() {
                        ui.set_running(false);
                    }
                }
            }
        }
    });
    let _ = ui.run();
    stop(&mut child.borrow_mut());
}

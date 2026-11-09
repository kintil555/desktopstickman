//! Panel pengaturan (Slint, gaya pixel GUI). Overlay = proses terpisah (`--overlay`), tetap jalan
//! saat panel ditutup; Start/Stop lewat pid file.
use crate::{
    config::{self, Anim, Body, Config, Inst, Item},
    shadow,
    stick::{Canvas, Stick, V},
    sys, tray,
};
use slint::{Color, ComponentHandle, Image, Rgba8Pixel, SharedPixelBuffer};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

mod ui {
    #![allow(deprecated, clippy::all, unused)]
    slint::include_modules!();
}
use ui::Panel;

const WHEEL: u32 = 80;
/// Sisi kanvas pratinjau (px); ditampilkan 1:1 di panel dengan sel `cell`.
const PV: u32 = 300;

fn hsv2rgb(h: f32, s: f32, v: f32) -> [u8; 3] {
    let h = h.rem_euclid(360.0) / 60.0;
    let c = v * s;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    [r, g, b].map(|k| ((k + m) * 255.0).round().clamp(0.0, 255.0) as u8)
}

fn rgb2hsv(c: [u8; 3]) -> (f32, f32, f32) {
    let [r, g, b] = c.map(|v| v as f32 / 255.0);
    let mx = r.max(g).max(b);
    let d = mx - r.min(g).min(b);
    let h = if d == 0.0 {
        0.0
    } else if mx == r {
        60.0 * ((g - b) / d).rem_euclid(6.0)
    } else if mx == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    (h, if mx == 0.0 { 0.0 } else { d / mx }, mx)
}

/// Color wheel (hue = sudut, saturasi = jari-jari), dirender sekali; ditampilkan 2x pixelated.
fn wheel_image() -> Image {
    let mut buf = SharedPixelBuffer::<Rgba8Pixel>::new(WHEEL, WHEEL);
    let (c, r) = (WHEEL as f32 / 2.0, WHEEL as f32 / 2.0);
    for (i, p) in buf.make_mut_slice().iter_mut().enumerate() {
        let (x, y) = (
            (i as u32 % WHEEL) as f32 + 0.5 - c,
            c - ((i as u32 / WHEEL) as f32 + 0.5),
        );
        let d = x.hypot(y) / r;
        if d <= 1.0 {
            let [rr, gg, bb] = hsv2rgb(y.atan2(x).to_degrees(), d, 1.0);
            *p = Rgba8Pixel::new(rr, gg, bb, 255);
        }
    }
    Image::from_rgba8(buf)
}

fn show_color(ui: &Panel, rgb: [u8; 3]) {
    ui.set_preview(Color::from_rgb_u8(rgb[0], rgb[1], rgb[2]));
}

/// Hitung ulang warna dari hue/sat/val (wheel + slider terang).
fn apply_hsv(ui: &Panel) {
    let rgb = hsv2rgb(ui.get_hue(), ui.get_sat(), ui.get_val());
    ui.set_hex(config::to_hex(rgb).into());
    show_color(ui, rgb);
}

fn read_body(ui: &Panel, b: &mut Body) {
    b.head_fill = ui.get_b_head_fill();
    b.head_size = ui.get_b_head_size();
    b.head_w = ui.get_b_head_w();
    b.limb_w = ui.get_b_limb_w();
    b.torso_w = ui.get_b_torso_w();
    b.outline = ui.get_b_outline();
    b.outline_color = config::parse_hex(&ui.get_b_outline_hex()).unwrap_or(b.outline_color);
    b.shade = ui.get_b_shade();
    b.shade_k = ui.get_b_shade_k();
    b.back_k = ui.get_b_back_k();
}

/// Config dari nilai panel saat ini (belum disimpan) khusus untuk pratinjau.
fn live_cfg(ui: &Panel, base: &Config) -> Config {
    let mut c = base.clone();
    c.color = hsv2rgb(ui.get_hue(), ui.get_sat(), ui.get_val());
    c.speed = ui.get_speed();
    c.cell = ui.get_cell().round() as u32;
    c.held = ui.get_held_id() as u32;
    c.shadow = ui.get_shadow();
    c.shadow_opacity = ui.get_shadow_opacity();
    c.shadow_dx = ui.get_shadow_dx().round() as i32;
    c.shadow_dy = ui.get_shadow_dy().round() as i32;
    c.shadow_blur = ui.get_shadow_blur().round() as u32;
    c.shadow_color = config::parse_hex(&ui.get_shadow_hex()).unwrap_or(c.shadow_color);
    read_body(ui, &mut c.body);
    c.sanitize();
    c
}

/// Satu frame pratinjau: sosok berlari di tempat di atas garis lantai, lengkap dengan shadow.
fn preview(
    ui: &Panel,
    base: &Config,
    st: &mut Stick,
    arts: &mut crate::held::Arts,
    dt: f32,
) -> Image {
    let cfg = live_cfg(ui, base);
    st.demo(dt, &cfg);
    crate::held::ensure(arts, &cfg.items);
    let mut cv = Canvas::new(PV, PV, cfg.cell);
    let o = V {
        x: PV as f32 / 2.0,
        y: PV as f32 * 0.8,
    };
    let row = (o.y / cv.cell) as usize;
    if row < cv.h {
        cv.px[row * cv.w..(row + 1) * cv.w].fill(0xFF55_5555);
    }
    let pose = st.pose();
    cv.draw(&pose, o, &cfg);
    if let Some(d) = crate::held::pick(&cfg.items, cfg.held, arts) {
        crate::held::draw(&mut cv, d, &pose, o, st.face, 0.0);
    }
    shadow::apply(&mut cv, &cfg);
    let mut buf = SharedPixelBuffer::<Rgba8Pixel>::new(cv.w as u32, cv.h as u32);
    for (d, &p) in buf.make_mut_slice().iter_mut().zip(&cv.px) {
        *d = Rgba8Pixel::new((p >> 16) as u8, (p >> 8) as u8, p as u8, (p >> 24) as u8);
    }
    Image::from_rgba8_premultiplied(buf)
}

fn load(ui: &Panel, c: &Config) {
    ui.set_speed(c.speed);
    ui.set_cell(c.cell as f32);
    ui.set_floor_offset(c.floor_offset as f32);
    ui.set_volume(c.volume);
    ui.set_jump(c.jump);
    ui.set_run(c.run);
    ui.set_fly(c.fly);
    ui.set_dust(c.dust);
    ui.set_hang(c.hang);
    ui.set_sound(c.sound);
    let (h, s, v) = rgb2hsv(c.color);
    ui.set_hue(h);
    ui.set_sat(s);
    ui.set_val(v);
    ui.set_hex(config::to_hex(c.color).into());
    show_color(ui, c.color);
    ui.set_shadow(c.shadow);
    ui.set_shadow_opacity(c.shadow_opacity);
    ui.set_shadow_dx(c.shadow_dx as f32);
    ui.set_shadow_dy(c.shadow_dy as f32);
    ui.set_shadow_blur(c.shadow_blur as f32);
    ui.set_shadow_hex(config::to_hex(c.shadow_color).into());
    let k = &c.skin;
    ui.set_skin_on(k.enabled);
    ui.set_skin_path(k.path.clone().into());
    ui.set_sk_cols(k.cols as i32);
    ui.set_sk_rows(k.rows as i32);
    ui.set_sk_scale(k.scale);
    ui.set_sk_seat(k.seat_scale);
    ui.set_sk_offx(k.off_x);
    ui.set_sk_offy(k.off_y);
    ui.set_sk_left(k.faces_left);
    ui.set_sk_tint(k.tint);
    ui.set_ai_row(k.idle.row as i32);
    ui.set_ai_fr(k.idle.frames as i32);
    ui.set_ai_fps(k.idle.fps as i32);
    ui.set_aw_row(k.walk.row as i32);
    ui.set_aw_fr(k.walk.frames as i32);
    ui.set_aw_fps(k.walk.fps as i32);
    ui.set_ar_row(k.run.row as i32);
    ui.set_ar_fr(k.run.frames as i32);
    ui.set_ar_fps(k.run.fps as i32);
    ui.set_aj_row(k.jump.row as i32);
    ui.set_aj_fr(k.jump.frames as i32);
    ui.set_aj_fps(k.jump.fps as i32);
    ui.set_autostart(c.autostart);
    ui.set_delay(c.startup_delay as f32);
    ui.set_bg_close(c.bg_on_close);
    let b = &c.body;
    ui.set_b_head_fill(b.head_fill);
    ui.set_b_head_size(b.head_size);
    ui.set_b_head_w(b.head_w);
    ui.set_b_limb_w(b.limb_w);
    ui.set_b_torso_w(b.torso_w);
    ui.set_b_outline(b.outline);
    ui.set_b_outline_hex(config::to_hex(b.outline_color).into());
    ui.set_b_shade(b.shade);
    ui.set_b_shade_k(b.shade_k);
    ui.set_b_back_k(b.back_k);
    set_keys(ui, c);
    update_grid(ui, &c.skin);
    ui.set_held_id(c.held as i32);
    ui.set_fight(c.fight);
    ui.set_hp(c.hp as f32);
    ui.set_dmg(c.dmg as f32);
    ui.set_team(c.team as f32);
    ui.set_respawn(c.respawn);
    ui.set_respawn_t(c.respawn_t);
    ui.set_atk_gap(c.atk_gap);
    ui.set_knock(c.knock);
    ui.set_auto_item(c.auto_item);
    refresh_insts(ui, c);
    refresh_items(ui, c);
}

/// Daftar item (tab Item) + editor untuk item terpilih.
fn refresh_items(ui: &Panel, c: &Config) {
    let (sel, worn) = (ui.get_sel_item(), ui.get_held_id());
    let mut rows = Vec::new();
    for it in &c.items {
        rows.push(ui::ItemRow {
            id: it.id as i32,
            name: it.name.clone().into(),
            sel: it.id as i32 == sel,
            worn: it.id as i32 == worn,
        });
    }
    ui.set_item_rows(slint::ModelRc::new(slint::VecModel::from(rows)));
    if let Some(it) = c.items.iter().find(|i| i.id as i32 == sel) {
        ui.set_item_back(it.hand == 1);
        ui.set_item_off_x(it.off_x);
        ui.set_item_off_y(it.off_y);
        ui.set_item_scale(it.scale);
        ui.set_item_angle(it.angle);
        ui.set_item_pvx(it.pivot_x);
        ui.set_item_pvy(it.pivot_y);
        ui.set_item_follow(it.follow_arm);
        ui.set_item_flip(it.flip);
    } else {
        ui.set_sel_item(0);
    }
}

/// Tambah item dari file PNG ke pustaka dan langsung dipakai stickman yang sedang diatur.
#[cfg_attr(not(windows), allow(dead_code))]
fn add_item(ui: &Panel, path: &str) {
    if crate::skin::load(path).is_none() {
        return; // bukan PNG yang valid
    }
    let mut c = Config::load();
    let id = c.next_item_id();
    let name = std::path::Path::new(path).file_stem().map_or_else(
        || format!("item {id}"),
        |s| s.to_string_lossy().into_owned(),
    );
    c.items.push(Item {
        id,
        name,
        path: path.to_string(),
        ..Item::default()
    });
    c.save();
    ui.set_sel_item(id as i32);
    ui.set_held_id(id as i32);
    let c = save_ui(ui);
    refresh_items(ui, &c);
}

/// Daftar stickman (tab Daftar) + editor untuk yang terpilih.
fn refresh_insts(ui: &Panel, c: &Config) {
    let sel = ui.get_sel_id();
    let mut rows = Vec::new();
    for i in &c.stickmen {
        rows.push(ui::InstRow {
            id: i.id as i32,
            name: format!("Stickman #{}", i.id).into(),
            color: Color::from_rgb_u8(i.cfg.color[0], i.cfg.color[1], i.cfg.color[2]),
            sel: i.id as i32 == sel,
        });
    }
    ui.set_insts(slint::ModelRc::new(slint::VecModel::from(rows)));
    if sel > 0 && c.stickmen.iter().any(|i| i.id as i32 == sel) {
        ui.set_target_name(format!("Stickman #{sel}").into());
    } else {
        ui.set_sel_id(0);
        ui.set_target_name("Default".into());
    }
}

/// Grid sprite sheet (tab Skin/Anim): dimuat ulang tiap pengaturan berubah.
fn update_grid(ui: &Panel, k: &config::Skin) {
    let img = crate::skin::grid_preview(k, 300, 190).map(|(w, h, px)| {
        let mut buf = SharedPixelBuffer::<Rgba8Pixel>::new(w as u32, h as u32);
        for (d, p) in buf.make_mut_slice().iter_mut().zip(px.chunks(4)) {
            *d = Rgba8Pixel::new(p[0], p[1], p[2], p[3]);
        }
        Image::from_rgba8(buf)
    });
    ui.set_grid_ok(img.is_some());
    if let Some(i) = img {
        ui.set_grid_img(i);
    }
}

fn set_keys(ui: &Panel, c: &Config) {
    ui.set_key_hang(c.keys.hang.clone().into());
    ui.set_key_jump(c.keys.jump.clone().into());
    ui.set_key_fly(c.keys.fly.clone().into());
    ui.set_key_hide(c.keys.hide.clone().into());
    ui.set_key_item(c.keys.item.clone().into());
}

/// Simpan isi panel: tab-tab mengatur stickman terpilih (atau Default bila tidak ada yang dipilih);
/// pengaturan sistem (tombol, lantai, autostart, ...) selalu global. Mengembalikan isi panel.
fn save_ui(ui: &Panel) -> Config {
    let c = collect(ui);
    let sel = ui.get_sel_id();
    let mut g = c.clone();
    if sel > 0 {
        g.copy_look(&Config::load());
        if let Some(i) = g.stickmen.iter_mut().find(|i| i.id as i32 == sel) {
            i.cfg.copy_look(&c);
        }
    }
    g.sanitize();
    g.save();
    c
}

/// Pengaturan yang tampil di tab: Default (id 0) atau milik stickman `id`.
fn effective(id: i32) -> Config {
    let mut c = Config::load();
    if let Some(i) = c.stickmen.iter().find(|i| i.id as i32 == id) {
        let look = i.cfg.clone();
        c.copy_look(&look);
    }
    c
}

fn collect(ui: &Panel) -> Config {
    let mut c = Config::load();
    c.speed = ui.get_speed();
    c.cell = ui.get_cell().round() as u32;
    c.held = ui.get_held_id() as u32;
    c.fight = ui.get_fight();
    c.hp = ui.get_hp().round() as u32;
    c.dmg = ui.get_dmg().round() as u32;
    c.team = ui.get_team().round() as u32;
    c.respawn = ui.get_respawn();
    c.respawn_t = ui.get_respawn_t();
    c.atk_gap = ui.get_atk_gap();
    c.knock = ui.get_knock();
    c.auto_item = ui.get_auto_item();
    c.floor_offset = ui.get_floor_offset().round() as i32;
    c.volume = ui.get_volume();
    c.jump = ui.get_jump();
    c.run = ui.get_run();
    c.fly = ui.get_fly();
    c.dust = ui.get_dust();
    c.hang = ui.get_hang();
    c.sound = ui.get_sound();
    c.color = config::parse_hex(&ui.get_hex()).unwrap_or(c.color);
    c.shadow = ui.get_shadow();
    c.shadow_opacity = ui.get_shadow_opacity();
    c.shadow_dx = ui.get_shadow_dx().round() as i32;
    c.shadow_dy = ui.get_shadow_dy().round() as i32;
    c.shadow_blur = ui.get_shadow_blur().round() as u32;
    c.shadow_color = config::parse_hex(&ui.get_shadow_hex()).unwrap_or(c.shadow_color);
    let k = &mut c.skin;
    k.enabled = ui.get_skin_on();
    k.path = ui.get_skin_path().trim().trim_matches('"').to_string();
    k.cols = ui.get_sk_cols() as u32;
    k.rows = ui.get_sk_rows() as u32;
    k.scale = ui.get_sk_scale();
    k.seat_scale = ui.get_sk_seat();
    k.off_x = ui.get_sk_offx();
    k.off_y = ui.get_sk_offy();
    k.faces_left = ui.get_sk_left();
    k.tint = ui.get_sk_tint();
    let set = |a: &mut Anim, row: i32, fr: i32, fps: i32| {
        (a.row, a.frames, a.fps) = (row as u32, fr as u32, fps as u32);
    };
    set(
        &mut k.idle,
        ui.get_ai_row(),
        ui.get_ai_fr(),
        ui.get_ai_fps(),
    );
    set(
        &mut k.walk,
        ui.get_aw_row(),
        ui.get_aw_fr(),
        ui.get_aw_fps(),
    );
    set(&mut k.run, ui.get_ar_row(), ui.get_ar_fr(), ui.get_ar_fps());
    set(
        &mut k.jump,
        ui.get_aj_row(),
        ui.get_aj_fr(),
        ui.get_aj_fps(),
    );
    c.autostart = ui.get_autostart();
    c.startup_delay = ui.get_delay().round() as u32;
    c.bg_on_close = ui.get_bg_close();
    read_body(ui, &mut c.body);
    c.keys.hang = ui.get_key_hang().to_string();
    c.keys.jump = ui.get_key_jump().to_string();
    c.keys.fly = ui.get_key_fly().to_string();
    c.keys.hide = ui.get_key_hide().to_string();
    c.keys.item = ui.get_key_item().to_string();
    c.sanitize();
    c
}

pub fn run() {
    let ui = Panel::new().expect("panel");
    load(&ui, &Config::load());
    ui.set_wheel(wheel_image());
    let on = sys::overlay_running();
    ui.set_running(on);
    ui.set_count(if on {
        Config::load().stickmen.len() as i32
    } else {
        0
    });

    ui.on_changed({
        let w = ui.as_weak();
        move || {
            if let Some(ui) = w.upgrade() {
                let c = save_ui(&ui);
                update_grid(&ui, &c.skin);
            }
        }
    });
    ui.on_wheel_pick({
        let w = ui.as_weak();
        move |x, y| {
            if let Some(ui) = w.upgrade() {
                let r = WHEEL as f32; // wheel tampil 2x: setengah lebar = WHEEL
                let (dx, dy) = (x - r, r - y);
                ui.set_hue(dy.atan2(dx).to_degrees().rem_euclid(360.0));
                ui.set_sat((dx.hypot(dy) / r).min(1.0));
                apply_hsv(&ui);
            }
        }
    });
    ui.on_color_edited({
        let w = ui.as_weak();
        move || {
            if let Some(ui) = w.upgrade() {
                apply_hsv(&ui);
            }
        }
    });
    ui.on_hex_accepted({
        let w = ui.as_weak();
        move || {
            let Some(ui) = w.upgrade() else { return };
            if let Some(rgb) = config::parse_hex(&ui.get_hex()) {
                let (h, s, v) = rgb2hsv(rgb);
                ui.set_hue(h);
                ui.set_sat(s);
                ui.set_val(v);
                ui.set_hex(config::to_hex(rgb).into());
                show_color(&ui, rgb);
                save_ui(&ui);
            }
        }
    });
    ui.on_shadow_hex_accepted({
        let w = ui.as_weak();
        move || {
            let Some(ui) = w.upgrade() else { return };
            if let Some(rgb) = config::parse_hex(&ui.get_shadow_hex()) {
                ui.set_shadow_hex(config::to_hex(rgb).into());
                save_ui(&ui);
            }
        }
    });
    ui.on_pick_skin({
        let w = ui.as_weak();
        move || {
            #[cfg(windows)]
            if let (Some(ui), Some(f)) = (
                w.upgrade(),
                rfd::FileDialog::new()
                    .add_filter("PNG", &["png"])
                    .pick_file(),
            ) {
                ui.set_skin_path(f.to_string_lossy().into_owned().into());
                ui.set_skin_on(true);
                let c = save_ui(&ui);
                update_grid(&ui, &c.skin);
            }
            #[cfg(not(windows))]
            let _ = &w; // non-Windows: ketik path langsung di kolom
        }
    });
    ui.on_autostart_changed({
        let w = ui.as_weak();
        move || {
            if let Some(ui) = w.upgrade() {
                let c = save_ui(&ui);
                sys::autostart(c.autostart);
            }
        }
    });
    ui.on_keys_accepted({
        let w = ui.as_weak();
        move || {
            if let Some(ui) = w.upgrade() {
                let c = save_ui(&ui);
                set_keys(&ui, &c);
            }
        }
    });
    ui.on_toggle({
        let w = ui.as_weak();
        move || {
            let Some(ui) = w.upgrade() else { return };
            if sys::overlay_running() {
                sys::stop_overlay();
                ui.set_running(false);
                ui.set_count(0);
            } else {
                // Start = satu proses overlay untuk semua stickman di daftar (kosong -> buat #1)
                save_ui(&ui);
                let mut c = Config::load();
                if c.stickmen.is_empty() {
                    let i = Inst::from_cfg(1, &c);
                    c.stickmen.push(i);
                    c.save();
                }
                let ok = sys::spawn(&["--overlay"]);
                ui.set_running(ok);
                ui.set_count(if ok { c.stickmen.len() as i32 } else { 0 });
                refresh_insts(&ui, &c);
            }
        }
    });
    // tambah stickman ke daftar (warna beda): overlay yang sudah jalan mengambilnya dari config,
    // bila belum jalan dimulai (maks. sys::MAX_SPAWN)
    ui.on_spawn_more({
        let w = ui.as_weak();
        move || {
            let Some(ui) = w.upgrade() else { return };
            save_ui(&ui);
            let mut c = Config::load();
            if c.stickmen.len() >= sys::MAX_SPAWN {
                return;
            }
            let id = c.next_id();
            let mut i = Inst::from_cfg(id, &c);
            if !c.stickmen.is_empty() {
                i.cfg.color = hsv2rgb(id as f32 * 137.5, 0.75, 1.0);
            }
            c.stickmen.push(i);
            c.save();
            if sys::overlay_running() || sys::spawn(&["--overlay"]) {
                ui.set_running(true);
                ui.set_count(c.stickmen.len() as i32);
            }
            refresh_insts(&ui, &c);
        }
    });
    ui.on_item_add({
        let w = ui.as_weak();
        move || {
            #[cfg(windows)]
            if let (Some(ui), Some(f)) = (
                w.upgrade(),
                rfd::FileDialog::new()
                    .add_filter("PNG", &["png"])
                    .pick_file(),
            ) {
                add_item(&ui, &f.to_string_lossy());
            }
            #[cfg(not(windows))]
            let _ = &w;
        }
    });
    ui.on_item_select({
        let w = ui.as_weak();
        move |id| {
            let Some(ui) = w.upgrade() else { return };
            ui.set_sel_item(id);
            refresh_items(&ui, &Config::load());
        }
    });
    // pakai / lepas item untuk stickman yang sedang diatur
    ui.on_item_wear({
        let w = ui.as_weak();
        move |id| {
            let Some(ui) = w.upgrade() else { return };
            ui.set_held_id(if ui.get_held_id() == id { 0 } else { id });
            let c = save_ui(&ui);
            refresh_items(&ui, &c);
        }
    });
    ui.on_item_delete({
        let w = ui.as_weak();
        move |id| {
            let Some(ui) = w.upgrade() else { return };
            let mut c = Config::load();
            c.items.retain(|i| i.id as i32 != id);
            // yang memakai item ini jadi tangan kosong
            if c.held as i32 == id {
                c.held = 0;
            }
            for i in &mut c.stickmen {
                if i.cfg.held as i32 == id {
                    i.cfg.held = 0;
                }
            }
            c.save();
            if ui.get_held_id() == id {
                ui.set_held_id(0);
            }
            if ui.get_sel_item() == id {
                ui.set_sel_item(0);
            }
            refresh_items(&ui, &c);
        }
    });
    ui.on_item_changed({
        let w = ui.as_weak();
        move || {
            let Some(ui) = w.upgrade() else { return };
            let sel = ui.get_sel_item();
            let mut c = Config::load();
            if let Some(it) = c.items.iter_mut().find(|i| i.id as i32 == sel) {
                it.hand = u32::from(ui.get_item_back());
                it.off_x = ui.get_item_off_x();
                it.off_y = ui.get_item_off_y();
                it.scale = ui.get_item_scale();
                it.angle = ui.get_item_angle();
                it.pivot_x = ui.get_item_pvx();
                it.pivot_y = ui.get_item_pvy();
                it.follow_arm = ui.get_item_follow();
                it.flip = ui.get_item_flip();
            }
            c.sanitize();
            c.save();
            refresh_items(&ui, &c);
        }
    });
    ui.on_inst_select({
        let w = ui.as_weak();
        move |id| {
            let Some(ui) = w.upgrade() else { return };
            ui.set_sel_id(id);
            load(&ui, &effective(id));
        }
    });
    // hapus stickman tertentu: keluar dari daftar, overlay menutup jendelanya
    ui.on_inst_delete({
        let w = ui.as_weak();
        move |id| {
            let Some(ui) = w.upgrade() else { return };
            let mut c = Config::load();
            c.stickmen.retain(|i| i.id as i32 != id);
            c.save();
            // overlay membaca config tiap 0,5 dtk; daftar kosong = overlay berhenti
            let on = !c.stickmen.is_empty() && sys::overlay_running();
            ui.set_running(on);
            ui.set_count(if on { c.stickmen.len() as i32 } else { 0 });
            if ui.get_sel_id() == id {
                ui.set_sel_id(0);
                load(&ui, &c);
            } else {
                refresh_insts(&ui, &c);
            }
        }
    });
    // perekam tombol: klik kotak lalu tekan kombinasi apa saja (mis. Alt+C); Esc = batal
    let rec: Rc<RefCell<Vec<u32>>> = Rc::default();
    ui.on_record({
        let (w, rec) = (ui.as_weak(), rec.clone());
        move |n| {
            let Some(ui) = w.upgrade() else { return };
            ui.set_recording(n);
            *rec.borrow_mut() = sys::held();
        }
    });
    let rec_timer = slint::Timer::default();
    let rec_tick = {
        let w = ui.as_weak();
        move || {
            let Some(ui) = w.upgrade() else { return };
            let n = ui.get_recording();
            if n == 0 {
                return;
            }
            if sys::key_down(0x1B) {
                ui.set_recording(0);
                return;
            }
            let Some(name) = sys::capture(&mut rec.borrow_mut()) else {
                return;
            };
            match n {
                1 => ui.set_key_hang(name.into()),
                2 => ui.set_key_jump(name.into()),
                3 => ui.set_key_fly(name.into()),
                4 => ui.set_key_hide(name.into()),
                _ => ui.set_key_item(name.into()),
            }
            ui.set_recording(0);
            let c = save_ui(&ui);
            set_keys(&ui, &c);
        }
    };
    rec_timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(40),
        rec_tick,
    );
    ui.on_quit_panel(|| {
        let _ = slint::quit_event_loop();
    });
    ui.window().on_close_requested(|| {
        let _ = slint::quit_event_loop();
        slint::CloseRequestResponse::HideWindow
    });
    // minimize -> sembunyikan ke tray (ikon tray hanya aktif bila tersedia, mis. Windows)
    let tray_rx = tray::install();
    let timer = slint::Timer::default();
    if let Some(rx) = tray_rx {
        let w = ui.as_weak();
        timer.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_millis(150),
            move || {
                let Some(ui) = w.upgrade() else { return };
                while let Ok(e) = rx.try_recv() {
                    match e {
                        tray::Ev::Show => {
                            let _ = ui.show();
                            ui.window().set_minimized(false);
                        }
                        tray::Ev::Quit => {
                            let _ = slint::quit_event_loop();
                        }
                    }
                }
                if ui.window().is_minimized() {
                    let _ = ui.hide();
                }
            },
        );
    }
    // pratinjau model (~30 fps), hanya saat jendela terlihat
    let pv_timer = slint::Timer::default();
    {
        let w = ui.as_weak();
        let mut base = Config::load();
        let mut mt = Config::mtime();
        let mut arts = crate::held::Arts::new();
        let mut st = Stick::new(0.0);
        let mut last = Instant::now();
        pv_timer.start(
            slint::TimerMode::Repeated,
            Duration::from_millis(33),
            move || {
                let Some(ui) = w.upgrade() else { return };
                let now = Instant::now();
                let dt = now.duration_since(last).as_secs_f32().min(0.1);
                last = now;
                if !ui.window().is_visible() || ui.window().is_minimized() {
                    return;
                }
                // pustaka item bisa berubah (tab Item): muat ulang bila config berubah
                let cur = Config::mtime();
                if cur != mt {
                    mt = cur;
                    base = Config::load();
                }
                ui.set_preview_img(preview(&ui, &base, &mut st, &mut arts, dt));
            },
        );
    }
    let _ = ui.show();
    let _ = slint::run_event_loop_until_quit();
    tray::remove();
    if !Config::load().bg_on_close {
        sys::stop_overlay();
    }
}

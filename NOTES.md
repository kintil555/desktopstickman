# NOTES untuk AI berikutnya

## Status
Milestone 1 selesai: overlay winit+softbuffer (src/main.rs), gait IK + rasterizer sel (src/stick.rs), config JSON `desktopstickman.json` (src/config.rs), state Idle/Walk/Run + lompat acak. Tes visual headless: `desktopstickman --dump out.ppm`.
Milestone 2 (DITULIS TAPI BELUM DI-BUILD/TES, cek `cargo build` + `--dump` dulu): debu (src/fx.rs: puff lari + burst mendarat), kendaraan terbang (src/veh.rs; sprite assets/*.rgba = RGBA mentah 59x59 dari PNG; flame prosedural; pemain duduk di-clip), state terbang di src/stick.rs (Fly::Up/Cruise/Down, jendela naik mengikuti `alt`). Tanpa tilt/asap/glow kendaraan.
Milestone 3 (JUGA BELUM DI-BUILD/TES): panel Slint (ui/panel.slint, src/panel.rs; tanpa argumen = panel, `--overlay` = overlay; Start/Stop = spawn/kill child, config disimpan ke desktopstickman.json saat slider dilepas/checkbox diubah), alpha per-piksel Windows (src/winalpha.rs: layered window + UpdateLayeredWindow, dipakai via src/present.rs; non-Windows tetap softbuffer). Config baru: sound, volume (belum dipakai; audio belum ada).
**Belum:** audio (rodio, fitur `audio`), tilt/asap/glow kendaraan, tooltip/quit (jendela click-through; berhenti via kill proses). clippy belum dijalankan lokal.

## Rencana (keputusan sudah diambil)
- Overlay: `winit` + `softbuffer`, jendela kecil (~320x160px) transparan, tanpa dekorasi, always-on-top, `set_cursor_hittest(false)`; jendela dipindah mengikuti stickman. Catatan: softbuffer di Windows mungkin tak mendukung alpha per-piksel; cek, kalau gagal pakai UpdateLayeredWindow.
- UI Slint = proses terpisah (tanpa argumen = panel; tombol Start menjalankan `--overlay` sebagai child, Stop = kill). Config JSON (warna, skala sel, kecepatan, volume, toggle fly/dust/suara, offset lantai/taskbar). Overlay polling mtime config.
- Audio: `rodio` (fitur `audio`; butuh libasound2-dev di Linux), sintesis sendiri: pulse/triangle/noise + bitcrush 6-bit. Suara: step, jump, land, click, jet loop.
- Perilaku: Idle/Walk/Run di lantai (taskbar), lompat, panggil kendaraan lalu terbang/melayang (bob) di ketinggian acak, mendarat + dust.
- Render per sel 2px (grid kecil, upscale).

## Konstanta dari HTML (satuan px asli, sel=2px)
S=1.25*1.02; R=S/2.4; HEADR=7.5S; NECK=2S; TORSO=18S; UA=FA=12S; TH=SH=14S; LEG=28S; A=LEG*.56; LIFT=LEG*.26; WALK=350; GRAV=2400
COL: F[238,122,28] B[184,84,26] HI[255,178,90] FS[214,100,14] BS[140,62,18] OL[40,20,10]
IK 2-tulang: ik(a,b,l1,l2,bend): d=clamp(|b-a|,|l1-l2|+.01,l1+l2-.01); an=acos((l1²+d²-l2²)/(2 l1 d)); titik = a + l1*(cos,sin)(atan2(dy,dx)+bend*an)
Gait: u=(ph+i*.5)%1; u<.5: wx=A(1-4u); else s=(u-.5)*2,e=s²(3-2s): wx=A(2e-1), wy=LIFT*sin(pi s). Hip y=-(LEG*(.94-.14*m) - R*(m*4|cos(2pi ph)|+idle*(3+3sin(2.4t)))).
Gambar: kaki/lengan belakang warna B, badan+kaki depan F, kepala lingkaran, lineWidth 4.
Kendaraan VEH: NZ[[11.5,53],[48,52]] nozzle, BP[30,34] kursi, JP[[11,46],[48,46]], CLIP polygon [[25,4],[33,4],[37,8],[40,14],[42,22],[42,28],[20,28],[20,20],[21,14],[22,8]] (pemain digambar hanya di dalam polygon), urutan layer: jb, body, pemain(clip), jf. Api prosedural: kerucut dari nozzle, L=5+9*thr, lebar 3.4+.9*thr, palet hot[234,253,255] br[173,255,252] cy[63,223,242] bl[94,186,250] vi[120,95,217]; asap abu [123,111,114]->[43,38,48] alpha .4->0.
RUNFX: curve1.6 sx[.5,1.4] sy[.22,.65] life[.26,.34] gap.05 drift[12,28] fleck.06. DUST: minH24 maxH420 curve.75 roll[1.1,9.1] spd[30,300] life[.18,.68] tail[1,6]; sprite DSPR (string array) ada di HTML.
Sumber lengkap: `Stickman Pixel.html` (cari: pose1, drawFig, spawnRun, DSPR, drawDust, vehFlames, SFX).

## Update (panel pixel GUI + fitur baru, BELUM di-build/tes — cek `cargo build`, `cargo fmt --check`, clippy)
- ui/panel.slint: gaya makespritewithclaude (aset ui/px = gui/button/field/fill/slider-head/web-header 2x nearest, font Groundhog). Tab: Umum, Warna (satu color wheel + terang + hex; badan belakang = `Config::back()` otomatis), Shadow, Model, Anim, Sistem.
- Config: `color` (alias color_f), shadow_*, autostart/startup_delay/bg_on_close, `skin` (sprite sheet PNG grid, anim per state, scale/offset/tint/faces_left). Config & pid kini di folder exe.
- src/shadow.rs (drop shadow geser+blur), src/skin.rs (PNG via crate `png`), src/sys.rs (pid, spawn terlepas, autostart: reg Run / .desktop, prioritas rendah).
- `--startup` = tidur `startup_delay` detik lalu overlay. Overlay: proses terpisah, ~30fps saat idle, skip present bila frame sama.
- Credit font: Pixel Font - Groundhog © 2022 Chevy Ray Johnston.
- Gantung di mouse (BELUM di-build): src/hang.rs (panjat hand-over-hand -> pendulum; kursor ke kanan = badan ke kiri, miring & ayun ikut kecepatan kursor), kursor/klik kanan dibaca via sys::cursor (GetCursorPos/GetAsyncKeyState, Windows saja). Klik kanan stickman = gantung, klik kanan lagi = lepas & jatuh. Skin kustom diputar di sekitar tangan (skin::draw_player param `hang`).
- State jatuh/mendarat (BELUM di-build): Stick.fall_t/arm0/land_t/land_k. Jatuh: kaki menekuk bertahap (tuck), tangan naik perlahan (0 -> 0.85 dalam ~0.6s), kecuali dari gantungan (arm0=1, sudah terangkat). Mendarat (lompat >=24px & drop dari gantungan): jongkok 0.4s (lutut menekuk, pinggul turun) + tangan terangkat lalu turun, diam selama animasi.
- Taskbar/tray (BELUM di-build, cek CI): overlay dibuat dengan `with_skip_taskbar(true)` (WS_EX_TOOLWINDOW setelah tampil tidak menghapus tombol taskbar). Panel: src/tray.rs (Shell_NotifyIconW di thread sendiri, event via channel); panel di-minimize -> `hide()` ke tray, klik kiri tray = buka, klik kanan = menu Buka/Keluar. Panel memakai `show()` + `run_event_loop_until_quit()`; tombol X dan Keluar = `quit_event_loop()`.
- Panel 940x700: kiri pengaturan (tab Umum/Warna/Shadow/Model/Skin/Anim/Tombol/Sistem), kanan pratinjau live (src/panel.rs `preview`, Stick::demo, ~30 fps, hanya saat jendela terlihat). Tombol bawah tinggi tetap 44px (sebelumnya meluas & menutupi isi). Tab Model = `Config.body` (kepala terisi/ukuran, tebal garis, outline, shading, terang sisi belakang) dipakai `Canvas::draw`. Tab Tombol = `Config.keys` (hang/jump/fly/hide, global lewat GetAsyncKeyState, kosong = nonaktif; sys::vk/key_down).
- VFX kendaraan (BELUM di-build): asap api (fx::Smoke, 70 maks), glow additive di sumbu api (veh::glow), tarikan partikel saat kendaraan dipanggil (fx::spawn_summon 0.18s) dan ledakan percikan saat turun (fx::spawn_burst). Belum: tilt kendaraan.
- Drop shadow pixel: topeng keras alpha datar; `shadow_blur` = pelebaran sel (default 0).

## Update (multi-spawn, ikut kursor, perekam tombol — BELUM di-build, cek CI)
- Banyak stickman = banyak proses `--overlay`; pid per proses `desktopstickman-<pid>.pid` (sys::overlay_pids, maks sys::MAX_SPAWN=12). Panel: tombol "Tambah +" = spawn lagi, Stop = kill semua. Posisi awal & seed acak per proses.
- Ikut kursor (hang): tahan klik kanan di stickman = ikut, lepas tombol = turun (hang_key=false). Tombol pintas (default `Alt+C`, hanya bila kursor di atas stickman) = toggle ikut/lepas (hang_key=true).
- Tombol pintas mendukung modifier (Ctrl/Alt/Shift+tombol, modifier harus persis): sys::parse/combo_down. Panel merekam tombol lewat polling GetAsyncKeyState (sys::held/capture, 40ms, Esc = batal, X = hapus), bukan ketik. Config lama dengan keys.hang kosong tetap kosong (default Alt+C hanya untuk config baru).
- Kendaraan turun: dust cloud (spawn_land) saat touchdown dihapus; tersisa percikan (spawn_burst).

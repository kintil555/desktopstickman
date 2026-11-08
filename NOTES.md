# NOTES untuk AI berikutnya

## Status
Hanya kerangka: Cargo.toml, stub src/main.rs, CI (.github/workflows/ci.yml), sprite kendaraan di assets/ (tsr_body/jb/jf.png, 59x59, diekstrak dari `Stickman Pixel.html`). **Engine belum ditulis.**

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

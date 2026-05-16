// oa-shell — the Tauri binary.
//
// Phase 1 acceptance target: a real HuCard ROM (e.g. Bonk's Adventure) rendering
// in the game window. Set the `OA_ROM` env var to the path of a `.pce` HuCard
// before launching; otherwise the core starts up with no game and the window
// stays black.

#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use oa_core::{Core, PortIndex};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log::info!("oa-shell starting (phase 1)");

    let rom_path = std::env::var("OA_ROM").ok();
    match &rom_path {
        Some(p) => log::info!("oa-shell: OA_ROM = {p}"),
        None => log::warn!(
            "oa-shell: OA_ROM not set — core will start with no game loaded; game window will be black. Set OA_ROM=<path/to/rom.pce> to play."
        ),
    }

    let running = Arc::new(AtomicBool::new(true));
    let tauri_running = running.clone();

    tauri::Builder::default()
        .setup({
            let running = running.clone();
            let rom_path = rom_path.clone();
            move |app| {
                let _library = tauri::WebviewWindowBuilder::new(
                    app,
                    "library",
                    tauri::WebviewUrl::App("index.html".into()),
                )
                .title("Overlooked Arcade")
                .inner_size(960.0, 640.0)
                .build()?;
                log::info!("oa-shell: library WebviewWindow built");

                let game = tauri::WindowBuilder::new(app, "game")
                    .title("Overlooked Arcade \u{2014} game")
                    .inner_size(768.0, 717.0)
                    .build()?;
                log::info!("oa-shell: game Window built");
                let game = Arc::new(game);

                std::thread::Builder::new()
                    .name("oa-emu-render".into())
                    .spawn({
                        let running = running.clone();
                        let game = game.clone();
                        let rom_path = rom_path.clone();
                        move || {
                            let raw_window = match game.window_handle() {
                                Ok(h) => h.as_raw(),
                                Err(e) => {
                                    log::error!("oa-shell: window_handle() failed: {e:?}");
                                    return;
                                }
                            };
                            let raw_display = match game.display_handle() {
                                Ok(h) => h.as_raw(),
                                Err(e) => {
                                    log::error!("oa-shell: display_handle() failed: {e:?}");
                                    return;
                                }
                            };
                            let initial_size = game
                                .inner_size()
                                .map(|s| (s.width, s.height))
                                .unwrap_or((768, 717));

                            run_emu_render(
                                running,
                                game,
                                raw_window,
                                raw_display,
                                initial_size,
                                rom_path,
                            );
                        }
                    })?;

                Ok(())
            }
        })
        .on_window_event(move |_window, event| {
            if matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
                tauri_running.store(false, Ordering::SeqCst);
            }
        })
        .run(tauri::generate_context!())
        .expect("tauri run failed");

    log::info!("oa-shell: tauri exited, signalling threads");
    running.store(false, Ordering::SeqCst);
    log::info!("oa-shell: bye");
}

fn run_emu_render(
    running: Arc<AtomicBool>,
    _window_holder: Arc<tauri::Window>,
    raw_window: raw_window_handle::RawWindowHandle,
    raw_display: raw_window_handle::RawDisplayHandle,
    initial_size: (u32, u32),
    rom_path: Option<String>,
) {
    use oa_pce::PceCore;

    let mut renderer = match unsafe { oa_render::Renderer::new(raw_window, raw_display, initial_size) } {
        Ok(r) => r,
        Err(e) => {
            log::error!("oa-render init failed: {e:?}");
            return;
        }
    };

    let mut input = {
        use oa_input::{GamepadButton, GamepadMapping, Keycode, KeyboardMapping};
        use oa_pce::buttons as pce;
        let port = oa_core::PortIndex::Port0;

        let mut kb = KeyboardMapping::empty();
        kb.bind(port, pce::UP,     Keycode::Up);
        kb.bind(port, pce::DOWN,   Keycode::Down);
        kb.bind(port, pce::LEFT,   Keycode::Left);
        kb.bind(port, pce::RIGHT,  Keycode::Right);
        kb.bind(port, pce::I,      Keycode::Z);
        kb.bind(port, pce::II,     Keycode::X);
        kb.bind(port, pce::RUN,    Keycode::Enter);
        kb.bind(port, pce::SELECT, Keycode::RShift);

        // Default PCE gamepad layout matches RetroArch's Beetle PCE Fast: south
        // face = II, east face = I, start = RUN, select = SELECT, dpad = dpad.
        let mut pad = GamepadMapping::empty();
        pad.bind(port, pce::UP,     GamepadButton::DPadUp);
        pad.bind(port, pce::DOWN,   GamepadButton::DPadDown);
        pad.bind(port, pce::LEFT,   GamepadButton::DPadLeft);
        pad.bind(port, pce::RIGHT,  GamepadButton::DPadRight);
        pad.bind(port, pce::I,      GamepadButton::East);
        pad.bind(port, pce::II,     GamepadButton::South);
        pad.bind(port, pce::RUN,    GamepadButton::Start);
        pad.bind(port, pce::SELECT, GamepadButton::Select);

        oa_input::InputPoller::with_mappings(kb, pad)
    };
    log::info!(
        "oa-shell: emu+render thread up; keyboard: \u{2191}\u{2193}\u{2190}\u{2192} = d-pad, Z = I, X = II, Enter = RUN, RShift = SELECT; gamepad: dpad + east=I / south=II / start=RUN / select=SELECT"
    );

    let mut core = PceCore::new();
    let timing = core.timing();
    log::info!(
        "oa-shell: PceCore timing = {}x{} @ {:.3} Hz, audio {} Hz",
        timing.width, timing.height, timing.fps, timing.sample_rate
    );

    let mut audio = match oa_audio::AudioSink::new(timing.sample_rate) {
        Ok(a) => {
            log::info!("oa-shell: audio sink up at {} Hz", a.sample_rate());
            Some(a)
        }
        Err(e) => {
            log::warn!("oa-shell: audio disabled ({e:?}); game will run silent");
            None
        }
    };

    if let Some(path) = rom_path.as_deref() {
        match std::fs::read(path) {
            Ok(bytes) => {
                log::info!("oa-shell: loaded {} bytes from {}", bytes.len(), path);
                match core.load_rom(&bytes) {
                    Ok(()) => log::info!("oa-shell: ROM accepted by PCE core; emulation will start"),
                    Err(e) => log::error!("oa-shell: ROM rejected: {e:?}"),
                }
            }
            Err(e) => {
                log::error!("oa-shell: failed to read ROM at {path}: {e:?}");
            }
        }
    }

    let frame_period = Duration::from_secs_f64(1.0 / timing.fps);
    let started = Instant::now();
    let mut next_frame = Instant::now();
    let mut frame_n: u64 = 0;
    let mut last_size = initial_size;

    while running.load(Ordering::SeqCst) {
        if let Ok(size) = _window_holder.inner_size() {
            let s = (size.width, size.height);
            if s != last_size && s.0 > 0 && s.1 > 0 {
                renderer.resize(s.0, s.1);
                last_size = s;
            }
        }

        // is_focused() returns false for native (no-WebView) Tauri Windows even
        // when the user is actively typing into them, so we can't rely on it for
        // input gating yet. Leave polling unconditionally on; tighten once we
        // route keyboard events through Tauri's event loop in Phase 2.
        input.set_enabled(true);
        core.set_input(PortIndex::Port0, input.poll(PortIndex::Port0));
        core.run_frame();
        renderer.present(core.framebuffer());

        // Pump audio: drain whatever the core produced this frame into the sink.
        // `drain_audio` borrows &mut self, so this has to come after `framebuffer()`.
        if let Some(sink) = audio.as_mut() {
            let samples = core.drain_audio();
            if !samples.is_empty() {
                sink.push(samples);
            }
        } else {
            let _ = core.drain_audio();
        }

        frame_n += 1;
        if frame_n % 120 == 0 {
            let fb = core.framebuffer();
            let elapsed = started.elapsed().as_secs_f32();
            let actual_fps = frame_n as f32 / elapsed;
            let (pushed, dropped) = audio.as_ref().map(|a| a.stats()).unwrap_or((0, 0));
            log::info!(
                "oa-shell: frame {} (~{:.1} fps); fb {}x{}; rom_loaded = {}; audio {}+{} (pushed+dropped)",
                frame_n, actual_fps, fb.width, fb.height, core.has_rom(), pushed, dropped
            );
        }

        next_frame += frame_period;
        let now = Instant::now();
        if next_frame > now {
            std::thread::sleep(next_frame - now);
        } else {
            next_frame = now;
        }
    }

    log::info!("oa-shell: emu+render thread stopping at frame {frame_n}");
}

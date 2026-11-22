mod player;
mod render;
mod world;

use player::Player;
use render::Renderer;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;
use world::Level;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    level: Level,
    player: Player,
    keys: HashSet<String>,
    grabbed: bool,
    last: Instant,
    log: Option<std::fs::File>,
    start: Instant,
    frame: u64,
    // Contador de FPS para el overlay de F3
    fps_frames: u32,
    fps_t: Instant,
    fps: u32,
    show_fps: bool,
    // Tiempo acumulado para los ticks fijos de simulacion
    tick_acc: f32,
    // Banco de pruebas: gira la camara por cuatro angulos y apunta los FPS
    bench: Option<Bench>,
}

struct Bench {
    started: bool,
    next: Instant,
    left: usize,
}

impl App {
    fn log(&mut self, msg: &str) {
        // Log solo durante los primeros 5 minutos
        if self.start.elapsed().as_secs() > 300 {
            return;
        }
        if let Some(f) = &mut self.log {
            use std::io::Write;
            let t = self.start.elapsed().as_secs_f32();
            let _ = writeln!(f, "[{:7.2}] {}", t, msg);
        }
    }
}

impl App {
    fn new() -> Self {
        let level = Level::new();
        let player = Player::new(&level);
        App {
            window: None,
            renderer: None,
            level,
            player,
            keys: HashSet::new(),
            grabbed: false,
            last: Instant::now(),
            // Traza de entrada solo para las mediciones (RD_INPUT_LOG=1): asi el
            // juego no deja ningun archivo suelto junto a los ejecutables.
            log: if std::env::var_os("RD_INPUT_LOG").is_some() {
                std::fs::File::create("input_log.txt").ok()
            } else {
                None
            },
            start: Instant::now(),
            frame: 0,
            fps_frames: 0,
            fps_t: Instant::now(),
            fps: 0,
            show_fps: false,
            tick_acc: 0.0,
            bench: if std::env::var("RD_BENCH").is_ok() {
                Some(Bench { started: false, next: Instant::now(), left: 5 })
            } else {
                None
            },
        }
    }

    fn raycast(&self, eye: [f32; 3]) -> Option<((i32, i32, i32), (i32, i32, i32))> {
        // Coordenadas de mundo puras: (x, y horizontales, z arriba). La camara
        // del original va 0.3 detras del ojo y pick() solo prueba bb.grow(3).
        // eye_pos_at devuelve (x, alto, fondo) para el renderer; el DDA y la
        // caja usan (x, fondo, alto). Sin este cambio se lanzaba el rayo con la
        // altura en el eje horizontal y la seleccion no acertaba el suelo.
        let yaw = self.player.y_rot.to_radians();
        let pitch = self.player.x_rot.to_radians();
        let (sy, cy) = yaw.sin_cos();
        let (sp, cp) = pitch.sin_cos();
        let mut d = [sy * cp, -cy * cp, sp];
        let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        d = [d[0] / l, d[1] / l, d[2] / l];
        let mut pos = [
            eye[0] - d[0] * 0.3,
            eye[1] - d[1] * 0.3,
            eye[2] - d[2] * 0.3,
        ];
        let limit = self.player.bbox().grown(3.0, 3.0, 3.0);
        let mut normal = [0i32, 0, 0];
        let mut t = 0.0f32;
        while t < 6.0 {
            if pos[0] < limit.x0 || pos[0] > limit.x1
                || pos[1] < limit.y0 || pos[1] > limit.y1
                || pos[2] < limit.z0 || pos[2] > limit.z1
            {
                return None;
            }
            let bx = pos[0].floor() as i32;
            let by = pos[1].floor() as i32;
            let bz = pos[2].floor() as i32;
            if self.level.is_solid(bx, by, bz) {
                return Some(((bx, by, bz), (normal[0], normal[1], normal[2])));
            }
            let step = [
                if d[0] > 0.0 { 1 } else { -1 },
                if d[1] > 0.0 { 1 } else { -1 },
                if d[2] > 0.0 { 1 } else { -1 },
            ];
            let mut tmax = [f32::INFINITY; 3];
            for i in 0..3 {
                if d[i].abs() > 1e-6 {
                    let next = if step[i] > 0 {
                        (pos[i].floor() + 1.0 - pos[i]) / d[i]
                    } else {
                        (pos[i] - pos[i].floor()) / -d[i]
                    };
                    tmax[i] = next;
                }
            }
            let axis = if tmax[0] < tmax[1] && tmax[0] < tmax[2] {
                0
            } else if tmax[1] < tmax[2] {
                1
            } else {
                2
            };
            normal = [0, 0, 0];
            normal[axis] = -step[axis];
            // avance garantizado: nunca 0 para no quedarnos atascados
            let dt = tmax[axis].max(1e-4);
            for i in 0..3 {
                pos[i] += d[i] * dt;
            }
            t += dt;
        }
        None
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // El banco abre la misma ventana que el original (1024x768 de cliente) y se
        // queda encima, para que las dos cifras se midan en igualdad de condiciones.
        let bench = self.bench.is_some();
        let size = if bench {
            (1024, 768)
        } else {
            render::tamano_inicial((1024, 768))
        };
        let window = Arc::new(
            event_loop
                .create_window(
                    winit::window::WindowAttributes::default()
                        .with_title("RubyDung RS")
                        .with_inner_size(winit::dpi::PhysicalSize::new(size.0, size.1)),
                )
                .unwrap(),
        );
        if bench {
            window.set_window_level(winit::window::WindowLevel::AlwaysOnTop);
        }
        window.focus_window();
        let mut renderer = pollster::block_on(Renderer::new(window.clone()));
        renderer.build_chunks(&self.level);
        self.renderer = Some(renderer);
        // Mouse.setGrabbed(true) del original: si el gestor de ventanas lo
        // permite, el primer clic ya rompe o coloca en vez de solo capturar.
        if window.set_cursor_grab(winit::window::CursorGrabMode::Locked).is_ok() {
            window.set_cursor_visible(false);
            self.grabbed = true;
        }
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(window) = &self.window else { return };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(r) = &mut self.renderer {
                    r.config.width = size.width.max(1);
                    r.config.height = size.height.max(1);
                    r.surface.configure(&r.device, &r.config);
                }
            }
            WindowEvent::KeyboardInput {
                event: KeyEvent { logical_key, state, .. },
                ..
            } => {
                if state == ElementState::Pressed && logical_key == Key::Named(NamedKey::Escape) {
                    event_loop.exit();
                    return;
                }
                // F3: muestra/oculta el contador de FPS (como el depurador del original)
                if state == ElementState::Pressed && logical_key == Key::Named(NamedKey::F3) {
                    self.show_fps = !self.show_fps;
                    self.log(&format!("FPS OVERLAY {}", self.show_fps));
                    return;
                }
                // Original: Return guarda el mundo en level.dat.
                if state == ElementState::Pressed && logical_key == Key::Named(NamedKey::Enter) {
                    let _ = self.level.save();
                    self.log("SAVE level.dat");
                    return;
                }
                let name = match &logical_key {
                    Key::Character(c) => c.to_string(),
                    Key::Named(NamedKey::Space) => " ".to_string(),
                    Key::Named(NamedKey::ArrowUp) => "arrowup".to_string(),
                    Key::Named(NamedKey::ArrowDown) => "arrowdown".to_string(),
                    Key::Named(NamedKey::ArrowLeft) => "arrowleft".to_string(),
                    Key::Named(NamedKey::ArrowRight) => "arrowright".to_string(),
                    // En el original la tecla Windows izquierda (219) tambien salta
                    Key::Named(NamedKey::Super) | Key::Named(NamedKey::Meta) => " ".to_string(),
                    _ => return,
                };
                match state {
                    ElementState::Pressed => {
                        if logical_key == Key::Named(NamedKey::Escape) {
                            event_loop.exit();
                            return;
                        }
                        if self.keys.insert(name.clone()) {
                            self.log(&format!("KEY DOWN {:?}", logical_key));
                        }
                    }
                    ElementState::Released => {
                        if self.keys.remove(&name) {
                            self.log(&format!("KEY UP {:?}", logical_key));
                        }
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if state == ElementState::Pressed {
                    if !self.grabbed {
                        if window
                            .set_cursor_grab(winit::window::CursorGrabMode::Locked)
                            .is_ok()
                        {
                            window.set_cursor_visible(false);
                            self.grabbed = true;
                        }
                        // El original ya tiene el raton capturado al arrancar: el
                        // primer clic tambien rompe/coloca, no solo agarra.
                    }
                    let a = (self.tick_acc * 60.0).clamp(0.0, 1.0);
                    let (ex, ey, ez) = self.player.eye_pos_at(a);
                    if let Some(((bx, by, bz), normal)) = self.raycast([ex, ez, ey]) {
                        self.log(&format!("CLICK {:?} hit=({},{},{}) n=({},{},{})", button, bx, by, bz, normal.0, normal.1, normal.2));
                        match button {
                            // Original: boton derecho (1) rompe, izquierdo (0) coloca
                            MouseButton::Right => {
                                self.level.set(bx, by, bz, world::AIR);
                                self.log(&format!("BREAK ({},{},{})", bx, by, bz));
                            }
                            MouseButton::Left => {
                                // Fiel al original: coloca sin comprobar solidez ni
                                // colision con el jugador; el setTile sustituye lo
                                // que hubiera en la cara contigua.
                                let (nx, ny, nz) = (bx + normal.0, by + normal.1, bz + normal.2);
                                self.level.set(nx, ny, nz, world::GRASS);
                                self.log(&format!("PLACE ({},{},{})", nx, ny, nz));
                            }
                            _ => {}
                        }
                        if let Some(r) = &mut self.renderer {
                            let cx = (bx as usize) / render::CHUNK;
                            let cy = (by as usize) / render::CHUNK;
                            for dx in -1..=1i32 {
                                for dy in -1..=1i32 {
                                    let nx = cx as i32 + dx;
                                    let ny = cy as i32 + dy;
                                    if nx >= 0 && ny >= 0
                                        && (nx as usize) < crate::world::WIDTH / render::CHUNK
                                        && (ny as usize) < crate::world::HEIGHT / render::CHUNK
                                    {
                                        r.rebuild_chunk(&self.level, nx as usize, ny as usize);
                                    }
                                }
                            }
                            // también el chunk de la celda donde se colocó el bloque
                            let (tx, ty) = (bx + normal.0, by + normal.1);
                            let tcx = (tx.max(0) as usize) / render::CHUNK;
                            let tcy = (ty.max(0) as usize) / render::CHUNK;
                            if tcx < crate::world::WIDTH / render::CHUNK && tcy < crate::world::HEIGHT / render::CHUNK {
                                r.rebuild_chunk(&self.level, tcx, tcy);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _device_id: winit::event::DeviceId, event: DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta } = event {
            if self.grabbed {
                self.log(&format!("MOUSE dx={:.1} dy={:.1}", delta.0, delta.1));
                self.player.turn(delta.0 as f32, delta.1 as f32);
            }
        }
    }

    // Original: destroy() guarda el mundo al cerrar el juego.
    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        let _ = self.level.save();
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let dt = now.duration_since(self.last).as_secs_f32().min(1.0);
        self.last = now;

        let f: f32 = if self.keys.contains("w") || self.keys.contains("arrowup") { 1.0 } else { 0.0 }
            - if self.keys.contains("s") || self.keys.contains("arrowdown") { 1.0 } else { 0.0 };
        let s: f32 = if self.keys.contains("d") || self.keys.contains("arrowright") { 1.0 } else { 0.0 }
            - if self.keys.contains("a") || self.keys.contains("arrowleft") { 1.0 } else { 0.0 };
        let jump = self.keys.contains(" ");
        // El original simula a ritmo FIJO: Timer(60.0f) y en run() hace
        // "for (t = 0; t < timer.ticks; t++) tick();". Un tick por frame hacia que
        // a 450 fps la fisica fuera 7.5 veces mas rapida (el "speedhack").
        self.tick_acc += dt;
        let mut steps = (self.tick_acc * 60.0) as i32;
        if steps > 100 {
            steps = 100;
        }
        self.tick_acc -= steps as f32 / 60.0;
        let reset = self.keys.contains("r") || self.keys.contains("R");
        for _ in 0..steps {
            // El original comprueba la R en cada tick: mantenida teletransporta
            // sin parar, no solo al pulsarla.
            self.player.tick(&self.level, f, s, jump, reset);
        }
        self.frame += 1;
        // FPS: media sobre ventanas de medio segundo, para que no baile el numero
        self.fps_frames += 1;
        let elapsed = self.fps_t.elapsed();
        if elapsed.as_millis() >= 500 {
            self.fps = (self.fps_frames as f64 * 1000.0 / elapsed.as_millis() as f64).round() as u32;
            self.fps_frames = 0;
            self.fps_t = Instant::now();
            // Util para medir sin depender del overlay (el log dura el primer minuto)
            self.log(&format!("FPS {}", self.fps));
        }
        // Banco: se queda en la vista de aparicion (el mismo sitio y el mismo
        // angulo que el original) y publica una unica medida, para que los dos
        // numeros del launcher sean comparables.
        if let Some(b) = &mut self.bench {
            if !b.started {
                b.started = true;
                b.next = Instant::now() + std::time::Duration::from_millis(1000);
            } else if Instant::now() >= b.next {
                println!("BENCH step={} yaw={:.0} fps={}", 5 - b.left, self.player.y_rot, self.fps);
                b.left -= 1;
                if b.left == 0 {
                    println!("BENCH DONE");
                    std::process::exit(0);
                }
                b.next = Instant::now() + std::time::Duration::from_millis(700);
            }
        }
        if self.frame % 20 == 0 {
            self.log(&format!(
                "POS x={:.2} y={:.2} z={:.2} yRot={:.1} xRot={:.1} onGround={} keys={:?}",
                self.player.x, self.player.y, self.player.z, self.player.y_rot, self.player.x_rot, self.player.on_ground, self.keys
            ));
        }

        self.player.careo_fijar();
        let a = (self.tick_acc * 60.0).clamp(0.0, 1.0);
        let (ex, ey, ez) = self.player.eye_pos_at(a);
        let hit = self.raycast([ex, ez, ey]);
        let yaw = self.player.y_rot.to_radians();
        let pitch = self.player.x_rot.to_radians();
        if let (Some(r), Some(w)) = (&mut self.renderer, &self.window) {
            let fps = if self.show_fps { Some(self.fps) } else { None };
            r.render([ex, ey, ez], yaw, pitch, hit, fps, &[], &[]);
            w.request_redraw();
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new();
    event_loop.run_app(&mut app).unwrap();
}

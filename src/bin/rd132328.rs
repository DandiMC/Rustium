// rd-132328 portado a Rust: la version anterior a la ya portada (rd-132211) y
// primera con un movil: el zombi. Level, Tile, LevelRenderer, Tesselator y AABB
// son identicos a rd-132211, asi que se reutilizan tal cual; lo nuevo es la capa
// de entidades (entity.rs) y la piel char.png para los modelos.

#[path = "../entity.rs"]
mod entity;
#[path = "../render.rs"]
mod render;
#[path = "../world.rs"]
mod world;

use entity::{PlayerE, Zombie};
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

const VERSION: &str = "rd-132328";

struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    level: Level,
    player: PlayerE,
    zombis: Vec<Zombie>,
    keys: HashSet<String>,
    grabbed: bool,
    last: Instant,
    log: Option<std::fs::File>,
    start: Instant,
    frame: u64,
    fps_frames: u32,
    fps_t: Instant,
    fps: u32,
    show_fps: bool,
    tick_acc: f32,
}

impl App {
    fn log(&mut self, msg: &str) {
        if self.start.elapsed().as_secs() > 300 {
            return;
        }
        if let Some(f) = &mut self.log {
            use std::io::Write;
            let t = self.start.elapsed().as_secs_f32();
            let _ = writeln!(f, "[{:7.2}] {}", t, msg);
        }
    }

    fn new() -> Self {
        let level = Level::new();
        let mut player = PlayerE::new(&level);
        // Atajo de pruebas: RD132328_POS="x,y" aparece en esa columna (cayendo).
        // Sin la variable el spawn es aleatorio, como el original.
        if let Ok(s) = std::env::var("RD132328_POS") {
            if let Some((a, b)) = s.split_once(',') {
                if let (Ok(x), Ok(y)) = (a.trim().parse::<f32>(), b.trim().parse::<f32>()) {
                    player.e.set_pos(x, y, world::DEPTH as f32 + 10.0);
                }
            }
        }
        // RubyDung.init del original: 100 zombis en (128, 0, 128)
        let mut zombis = Vec::with_capacity(100);
        for _ in 0..100 {
            zombis.push(Zombie::new(&level, 128.0, 128.0, 0.0));
        }
        App {
            window: None,
            renderer: None,
            level,
            player,
            zombis,
            keys: HashSet::new(),
            grabbed: false,
            last: Instant::now(),
            log: std::fs::File::create("input_log_132328.txt").ok(),
            start: Instant::now(),
            frame: 0,
            fps_frames: 0,
            fps_t: Instant::now(),
            fps: 0,
            show_fps: false,
            tick_acc: 0.0,
        }
    }

    // Raycast desde el ojo del jugador (aqui la z de la entidad ya es el ojo).
    fn raycast(&self) -> Option<((i32, i32, i32), (i32, i32, i32))> {
        let e = &self.player.e;
        let mut pos = [e.x, e.y, e.z];
        let yaw = e.y_rot.to_radians();
        let pitch = e.x_rot.to_radians();
        let (sy, cy) = yaw.sin_cos();
        let (sp, cp) = pitch.sin_cos();
        let mut d = [sy * cp, -cy * cp, sp];
        let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        d = [d[0] / l, d[1] / l, d[2] / l];
        let mut normal = [0i32, 0, 0];
        let mut t = 0.0f32;
        while t < 6.0 {
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
        let window = Arc::new(
            event_loop
                .create_window(
                    winit::window::WindowAttributes::default()
                        .with_title("RubyDung RS (rd-132328)")
                        .with_inner_size(winit::dpi::PhysicalSize::new(1024, 768)),
                )
                .unwrap(),
        );
        let mut renderer = pollster::block_on(Renderer::new(window.clone()));
        renderer.build_chunks(&self.level);
        // Piel de los modelos: char.png de la version original instalada
        match render::cargar_png_del_jar(VERSION, "char.png") {
            Some((w, h, rgba)) => {
                renderer.pon_piel(w, h, &rgba);
                println!("piel char.png cargada: {w}x{h}");
            }
            None => eprintln!("aviso: no encontre char.png en {VERSION}.jar; los zombis saldran sin piel"),
        }
        self.renderer = Some(renderer);
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
                if state == ElementState::Pressed && logical_key == Key::Named(NamedKey::F3) {
                    self.show_fps = !self.show_fps;
                    return;
                }
                let name = match &logical_key {
                    Key::Character(c) => c.to_string(),
                    Key::Named(NamedKey::Space) => " ".to_string(),
                    Key::Named(NamedKey::Super) | Key::Named(NamedKey::Meta) => " ".to_string(),
                    _ => return,
                };
                match state {
                    ElementState::Pressed => {
                        if name == "r" {
                            self.player.e.reset_pos(&self.level);
                        }
                        self.keys.insert(name);
                    }
                    ElementState::Released => {
                        self.keys.remove(&name);
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if state == ElementState::Pressed {
                    if !self.grabbed {
                        let _ = window.set_cursor_grab(winit::window::CursorGrabMode::Locked);
                        window.set_cursor_visible(false);
                        self.grabbed = true;
                        return;
                    }
                    if let Some(((bx, by, bz), normal)) = self.raycast() {
                        self.log(&format!("CLICK {:?} hit=({},{},{}) n=({},{},{})", button, bx, by, bz, normal.0, normal.1, normal.2));
                        match button {
                            // Original: boton derecho rompe, izquierdo coloca
                            MouseButton::Right => {
                                self.level.set(bx, by, bz, world::AIR);
                                self.log(&format!("BREAK ({},{},{})", bx, by, bz));
                            }
                            MouseButton::Left => {
                                let (nx, ny, nz) = (bx + normal.0, by + normal.1, bz + normal.2);
                                if !self.level.is_solid(nx, ny, nz) {
                                    self.level.set(nx, ny, nz, world::GRASS);
                                    self.log(&format!("PLACE ({},{},{})", nx, ny, nz));
                                }
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
                                    if nx >= 0
                                        && ny >= 0
                                        && (nx as usize) < world::WIDTH / render::CHUNK
                                        && (ny as usize) < world::HEIGHT / render::CHUNK
                                    {
                                        r.rebuild_chunk(&self.level, nx as usize, ny as usize);
                                    }
                                }
                            }
                            let (tx, ty) = (bx + normal.0, by + normal.1);
                            let tcx = (tx.max(0) as usize) / render::CHUNK;
                            let tcy = (ty.max(0) as usize) / render::CHUNK;
                            if tcx < world::WIDTH / render::CHUNK && tcy < world::HEIGHT / render::CHUNK {
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
                self.player.e.turn(delta.0 as f32, delta.1 as f32);
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let dt = now.duration_since(self.last).as_secs_f32().min(1.0);
        self.last = now;

        let f: f32 = if self.keys.contains("w") { 1.0 } else { 0.0 } - if self.keys.contains("s") { 1.0 } else { 0.0 };
        let s: f32 = if self.keys.contains("d") { 1.0 } else { 0.0 } - if self.keys.contains("a") { 1.0 } else { 0.0 };
        let jump = self.keys.contains(" ");
        // Simulacion a ritmo fijo, como Timer(60) en el original
        self.tick_acc += dt;
        let mut steps = (self.tick_acc * 60.0) as i32;
        if steps > 100 {
            steps = 100;
        }
        self.tick_acc -= steps as f32 / 60.0;
        for _ in 0..steps {
            for z in self.zombis.iter_mut() {
                z.tick(&self.level);
            }
            self.player.tick(&self.level, f, s, jump);
        }
        self.frame += 1;

        self.fps_frames += 1;
        let elapsed = self.fps_t.elapsed();
        if elapsed.as_millis() >= 500 {
            self.fps = (self.fps_frames as f64 * 1000.0 / elapsed.as_millis() as f64).round() as u32;
            self.fps_frames = 0;
            self.fps_t = Instant::now();
            self.log(&format!("FPS {}", self.fps));
        }
        if self.frame % 20 == 0 {
            let e = &self.player.e;
            self.log(&format!(
                "POS x={:.2} y={:.2} z={:.2} yRot={:.1} xRot={:.1} onGround={}",
                e.x, e.y, e.z, e.y_rot, e.x_rot, e.on_ground
            ));
        }

        let hit = self.raycast();
        let e = &self.player.e;
        let yaw = e.y_rot.to_radians();
        let pitch = e.x_rot.to_radians();
        let (sy, cy) = yaw.sin_cos();
        let (sp, cp) = pitch.sin_cos();
        // moveCameraToPlayer del original: glTranslatef(0,0,-0.3) DESPUES de
        // rotar, o sea el ojo va 0.3 por delante del punto del jugador.
        // Coords de camara del motor: (x, arriba, fondo)
        let fwd = [sy * cp, sp, -cy * cp];
        let eye = [
            e.x + fwd[0] * 0.3,
            e.z + fwd[1] * 0.3,
            e.y + fwd[2] * 0.3,
        ];

        // Modelos de todos los zombis en un solo lote. `a` es la fraccion de tick
        // que sobra, como el partialTicks del original: sin ella la posicion salta
        // a 60 Hz aunque se dibuje a 150 y el caminar se ve a tirones.
        let a = (self.tick_acc * 60.0).clamp(0.0, 1.0);
        let mut verts: Vec<[f32; 5]> = Vec::with_capacity(self.zombis.len() * 216);
        for z in &self.zombis {
            z.vertices(a, &mut verts);
        }

        let fps = if self.show_fps { Some(self.fps) } else { None };
        if let (Some(r), Some(w)) = (&mut self.renderer, &self.window) {
            r.render(eye, yaw, pitch, hit, fps, &verts);
            w.request_redraw();
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new();
    event_loop.run_app(&mut app).unwrap();
}

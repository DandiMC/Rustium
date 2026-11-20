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
    // Arnes de careo: congelar la simulacion y fijar el tiempo de animacion.
    freeze: bool,
    fixed_time: Option<f64>,
    // Solo careo: mantiene el rumbo de los zombis para comparar el caminar.
    fixed_zombie_rot: Option<f32>,
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
        // Atajo de pruebas: RD132328_POS="x,fondo" aparece en esa columna
        // (cayendo); con tres valores "x,fondo,alto" se fija la posicion exacta.
        // Sin la variable el spawn es aleatorio, como el original.
        if let Ok(s) = std::env::var("RD132328_POS") {
            let n: Vec<f32> = s.split(',').filter_map(|t| t.trim().parse().ok()).collect();
            match n.len() {
                2 => player.e.set_pos(n[0], n[1], world::DEPTH as f32 + 10.0),
                3 => player.e.set_pos(n[0], n[1], n[2]),
                _ => {}
            }
        }
        if let Ok(s) = std::env::var("RD132328_ROT") {
            let n: Vec<f32> = s.split(',').filter_map(|t| t.trim().parse().ok()).collect();
            if n.len() == 2 {
                player.e.y_rot = n[0];
                player.e.x_rot = n[1];
            }
        }
        // RubyDung.init del original: 100 zombis en (128, 0, 128).
        // RD132328_ZOMBIES y RD132328_ZOMBIE son solo para el careo: permiten
        // dejar el mundo sin zombis o poner uno quieto en una pose concreta.
        let n_zombis = std::env::var("RD132328_ZOMBIES")
            .ok()
            .and_then(|v| v.trim().parse::<usize>().ok())
            .unwrap_or(100);
        let mut zombis = Vec::with_capacity(n_zombis + 1);
        for _ in 0..n_zombis {
            zombis.push(Zombie::new(&level, 128.0, 128.0, 0.0));
        }
        if let Ok(s) = std::env::var("RD132328_ZOMBIE") {
            let n: Vec<f32> = s.split(',').filter_map(|t| t.trim().parse().ok()).collect();
            if n.len() >= 3 {
                let mut z = Zombie::new(&level, n[0], n[1], n[2]);
                z.e.set_pos(n[0], n[1], n[2]);
                // Sin esto el render interpola desde el resetPos aleatorio.
                z.e.xo = n[0];
                z.e.yo = n[1];
                z.e.zo = n[2];
                z.rot = n.get(3).copied().unwrap_or(0.0);
                z.time_offs = 0.0;
                if let Ok(v) = std::env::var("RD132328_ROT_A") {
                    if let Ok(a) = v.trim().parse::<f32>() {
                        z.rot_a = a;
                    }
                }
                zombis.push(z);
            }
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
            freeze: std::env::var_os("RD132328_FREEZE").is_some(),
            fixed_time: std::env::var("RD132328_TIME").ok().and_then(|v| v.trim().parse().ok()),
            fixed_zombie_rot: std::env::var("RD132328_ZOMBIE_ROT")
                .ok()
                .and_then(|v| v.trim().parse().ok()),
        }
    }

    // Posicion interpolada del jugador, como moveCameraToPlayer(a) del original.
    // Sin interpolar, la camara avanza a tirones de 60 Hz aunque se dibuje a 200.
    fn camera_player(&self) -> ([f32; 3], [f32; 3], f32, f32) {
        let e = &self.player.e;
        let a = (self.tick_acc * 60.0).clamp(0.0, 1.0);
        let px = e.xo + (e.x - e.xo) * a;
        let py = e.yo + (e.y - e.yo) * a;
        let pz = e.zo + (e.z - e.zo) * a;
        let yaw = e.y_rot.to_radians();
        let pitch = e.x_rot.to_radians();
        let (sy, cy) = yaw.sin_cos();
        let (sp, cp) = pitch.sin_cos();
        // Direccion en coordenadas de mundo: x, fondo, alto.
        let d = [sy * cp, -cy * cp, sp];
        ([px, py, pz], d, yaw, pitch)
    }

    // pick() del original: la camara va 0.3 bloques DETRAS del ojo
    // (glTranslatef(0,0,-0.3)) y OpenGL solo prueba los bloques dentro de
    // player.bb.grow(3,3,3). Aqui se hace lo mismo con un DDA: se empieza en la
    // camara y se para al salir de esa caja. Antes llegaba 6 bloques, asi que se
    // podia romper mas lejos que en el original.
    fn raycast(&self) -> Option<((i32, i32, i32), (i32, i32, i32))> {
        let ([px, py, pz], d, _, _) = self.camera_player();
        let eye = [px - d[0] * 0.3, py - d[1] * 0.3, pz - d[2] * 0.3];
        self.raycast_from(eye, d)
    }

    fn raycast_from(
        &self,
        mut pos: [f32; 3],
        d: [f32; 3],
    ) -> Option<((i32, i32, i32), (i32, i32, i32))> {
        let bb = self.player.e.bb;
        let grow = 3.0f32;
        let x0 = (bb.x0 - grow).floor() as i32;
        let x1 = (bb.x1 + grow).floor() as i32;
        let y0 = (bb.y0 - grow).floor() as i32;
        let y1 = (bb.y1 + grow).floor() as i32;
        let z0 = (bb.z0 - grow).floor() as i32;
        let z1 = (bb.z1 + grow).floor() as i32;
        let mut normal = [0i32, 0, 0];
        let mut t = 0.0f32;
        loop {
            let bx = pos[0].floor() as i32;
            let by = pos[1].floor() as i32;
            let bz = pos[2].floor() as i32;
            if bx < x0 || bx > x1 || by < y0 || by > y1 || bz < z0 || bz > z1 {
                return None;
            }
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
            if t > 32.0 {
                return None;
            }
        }
    }

    // Arnes de careo: RD132328_FIJA=1 fija posicion y rotacion en cada frame.
    fn careo_fijar(&mut self) {
        if std::env::var_os("RD132328_FIJA").is_none() {
            return;
        }
        if let Ok(s) = std::env::var("RD132328_POS") {
            let n: Vec<f32> = s.split(',').filter_map(|t| t.trim().parse().ok()).collect();
            if n.len() == 3 {
                self.player.e.x = n[0];
                self.player.e.y = n[1];
                self.player.e.z = n[2];
                self.player.e.xo = n[0];
                self.player.e.yo = n[1];
                self.player.e.zo = n[2];
                self.player.e.xd = 0.0;
                self.player.e.yd = 0.0;
                self.player.e.zd = 0.0;
                // La caja tambien va con la camara: el pick original limita a
                // player.bb.grow(3), y sin esto el careo usaba la caja del spawn.
                self.player.e.bb = world::BlockBox {
                    x0: n[0] - 0.3,
                    x1: n[0] + 0.3,
                    y0: n[1] - 0.3,
                    y1: n[1] + 0.3,
                    z0: n[2] - 1.62,
                    z1: n[2] + 0.18,
                };
            }
        }
        if let Ok(s) = std::env::var("RD132328_ROT") {
            let n: Vec<f32> = s.split(',').filter_map(|t| t.trim().parse().ok()).collect();
            if n.len() == 2 {
                self.player.e.y_rot = n[0];
                self.player.e.x_rot = n[1];
            }
        }
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
        // Mouse.setGrabbed(true) del original. Si el gestor de ventanas no lo
        // permite todavia, se reintenta en el primer clic.
        if window.set_cursor_grab(winit::window::CursorGrabMode::Locked).is_ok() {
            window.set_cursor_visible(false);
            self.grabbed = true;
        }
        // Esta version tiene su propio terrain.png (la hierba es un poco mas
        // clara que en rd-132211); sin esto el renderer usaria el atlas viejo.
        std::env::set_var("RD_ATLAS_VERSION", VERSION);
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
                // Original: Return guarda el mundo en level.dat.
                if state == ElementState::Pressed && logical_key == Key::Named(NamedKey::Enter) {
                    let _ = self.level.save();
                    self.log("SAVE level.dat");
                    return;
                }
                let name = match &logical_key {
                    Key::Character(c) => c.to_string(),
                    Key::Named(NamedKey::Space) => " ".to_string(),
                    // La tecla Windows izquierda (219) tambien salta, como el original.
                    Key::Named(NamedKey::Super) | Key::Named(NamedKey::Meta) => " ".to_string(),
                    // El original acepta flechas ademas de WASD (200/208/203/205).
                    Key::Named(NamedKey::ArrowUp) => "w".to_string(),
                    Key::Named(NamedKey::ArrowDown) => "s".to_string(),
                    Key::Named(NamedKey::ArrowLeft) => "a".to_string(),
                    Key::Named(NamedKey::ArrowRight) => "d".to_string(),
                    _ => return,
                };
                match state {
                    ElementState::Pressed => {
                        self.keys.insert(name);
                    }
                    ElementState::Released => {
                        self.keys.remove(&name);
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if state == ElementState::Pressed {
                    // El original agarra el raton en init(): el primer clic ya
                    // rompe o coloca. Si el sistema no deja agarrarlo antes de
                    // tener el foco, se agarra ahora pero el clic cuenta igual.
                    if !self.grabbed {
                        if window.set_cursor_grab(winit::window::CursorGrabMode::Locked).is_ok() {
                            window.set_cursor_visible(false);
                            self.grabbed = true;
                        }
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
                                // Fiel al original: sin comprobar solidez ni
                                // colision con el jugador.
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

    // Original: destroy() guarda el mundo al cerrar el juego.
    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        let _ = self.level.save();
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
            if !self.freeze {
                for z in self.zombis.iter_mut() {
                    z.tick(&self.level);
                }
                if let Some(rot) = self.fixed_zombie_rot {
                    for z in self.zombis.iter_mut() {
                        z.rot = rot;
                        z.rot_a = 0.0;
                    }
                }
            }
            // El original comprueba la R en cada tick: mantenida teletransporta
            // sin parar, no solo al pulsarla.
            if self.keys.contains("r") {
                self.player.e.reset_pos(&self.level);
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
            if let Some(z) = self.zombis.last() {
                self.log(&format!(
                    "Z x={:.2} y={:.2} z={:.2} rot={:.1} onGround={}",
                    z.e.x, z.e.y, z.e.z, z.rot, z.e.on_ground
                ));
            }
        }

        self.careo_fijar();
        // La camara del original interpola la posicion del jugador con `a`
        // (moveCameraToPlayer(a)). El render anade el adelanto de 0.3; el pick usa
        // la camara ya adelantada, por eso aqui se calcula aparte.
        let ([px, py, pz], dir, yaw, pitch) = self.camera_player();
        if std::env::var_os("RD132328_HIT_LOG").is_some() {
            self.log(&format!(
                "CAM x={:.2} y={:.2} z={:.2} dir={:?} bb=({:.2},{:.2},{:.2})-({:.2},{:.2},{:.2})",
                px, py, pz, dir,
                self.player.e.bb.x0, self.player.e.bb.y0, self.player.e.bb.z0,
                self.player.e.bb.x1, self.player.e.bb.y1, self.player.e.bb.z1
            ));
        }
        let hit = self.raycast_from(
            [px - dir[0] * 0.3, py - dir[1] * 0.3, pz - dir[2] * 0.3],
            dir,
        );
        let eye = [px, pz, py];
        if std::env::var_os("RD132328_HIT_LOG").is_some() {
            self.log(&format!("HIT {:?}", hit));
        }

        // Modelos de todos los zombis en un solo lote. `a` es la fraccion de tick
        // que sobra, como el partialTicks del original: sin ella la posicion salta
        // a 60 Hz aunque se dibuje a 150 y el caminar se ve a tirones.
        let a = (self.tick_acc * 60.0).clamp(0.0, 1.0);
        let mut verts: Vec<[f32; 5]> = Vec::with_capacity(self.zombis.len() * 216);
        for z in &self.zombis {
            if let Some(t) = self.fixed_time {
                z.vertices_at(t, a, &mut verts);
            } else {
                z.vertices(a, &mut verts);
            }
        }

        let fps = if self.show_fps { Some(self.fps) } else { None };
        if let (Some(r), Some(w)) = (&mut self.renderer, &self.window) {
            r.render(eye, yaw, pitch, hit, fps, &verts, &[]);
            w.request_redraw();
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new();
    event_loop.run_app(&mut app).unwrap();
}

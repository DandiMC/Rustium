// Juego de la familia rd-160052 / rd-20090515 / rd-161348 (RubyDung).
//
// Es el mismo codigo para las tres: rd-161348 es el mismo jar que rd-20090515 y
// rd-160052 solo se diferencia en que no define el tile "Bush", que ni el
// generador ni el jugador colocan nunca.
//
// Fiel a RubyDung.java: 10 zombis al empezar, elegir bloque con 1/2/3/4/6,
// G suelta un zombi, Enter guarda level.dat, clic derecho rompe (con 64
// particulas) y el izquierdo coloca el bloque elegido.

use super::level::{Nivel, DEPTH, HEIGHT, WIDTH};
use super::mesh;
use super::particula::{self, Particula};
use super::tiles;
use crate::entity::Zombie;
use crate::render::{Renderer, Vertex};
use crate::world::BlockBox;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

const SPAWN_SEED: i64 = 12345;

pub fn correr(version: &'static str) {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new(version);
    event_loop.run_app(&mut app).unwrap();
}

struct Jugador {
    x: f32,
    y: f32,
    z: f32,
    xd: f32,
    yd: f32,
    zd: f32,
    y_rot: f32,
    x_rot: f32,
    on_ground: bool,
}

impl Jugador {
    fn new() -> Self {
        // RD_ROT="yaw,pitch" fija la rotacion inicial (comodidad para capturas)
        let (mut y_rot, mut x_rot) = (0.0f32, 0.0f32);
        if let Ok(v) = std::env::var("RD_ROT") {
            let mut it = v.split(',');
            if let (Some(a), Some(b)) = (it.next(), it.next()) {
                y_rot = a.trim().parse().unwrap_or(0.0);
                x_rot = b.trim().parse().unwrap_or(0.0);
            }
        }
        // RD_POS="x,fondo,alto" coloca la camara donde diga (careo)
        let (mut px, mut py, mut pz) = (WIDTH as f32 / 2.0, HEIGHT as f32 / 2.0, DEPTH as f32 + 10.0);
        if let Ok(v) = std::env::var("RD_POS") {
            let n: Vec<f32> = v.split(',').filter_map(|t| t.trim().parse().ok()).collect();
            if n.len() == 3 {
                px = n[0];
                py = n[1];
                pz = n[2];
            }
        }
        Jugador {
            x: px,
            y: py,
            z: pz,
            xd: 0.0, yd: 0.0, zd: 0.0,
            y_rot,
            x_rot,
            on_ground: false,
        }
    }

    fn bbox(&self) -> BlockBox {
        BlockBox {
            x0: self.x - 0.3, x1: self.x + 0.3,
            y0: self.y - 0.3, y1: self.y + 0.3,
            z0: self.z - 1.62, z1: self.z + 0.18,
        }
    }

    fn mover(&mut self, xa: f32, ya: f32, za: f32, nivel: &Nivel) {
        let (xa0, ya0, za0) = (xa, ya, za);
        let area = self.bbox().expanded(xa, ya, za);
        let cubes = nivel.cubes(&area);
        let mut bb = self.bbox();

        let mut dz = za;
        for c in &cubes { dz = c.clip_z(&bb, dz); }
        bb.move_by(0.0, 0.0, dz);

        let mut dx = xa;
        for c in &cubes { dx = c.clip_x(&bb, dx); }
        bb.move_by(dx, 0.0, 0.0);

        let mut dy = ya;
        for c in &cubes { dy = c.clip_y(&bb, dy); }
        bb.move_by(0.0, dy, 0.0);

        self.on_ground = za0 != dz && za0 < 0.0;
        if xa0 != dx { self.xd = 0.0; }
        if za0 != dz { self.zd = 0.0; }
        if ya0 != dy { self.yd = 0.0; }

        self.x = (bb.x0 + bb.x1) / 2.0;
        self.y = (bb.y0 + bb.y1) / 2.0;
        self.z = bb.z0 + 1.62;
    }

    fn move_relative(&mut self, xa: f32, za: f32, speed: f32) {
        let dist = (xa * xa + za * za).sqrt();
        if dist < 0.01 { return; }
        let d = speed / dist;
        let yr = self.y_rot.to_radians();
        self.xd += (xa * yr.sin() + za * yr.cos()) * d;
        self.yd += (-xa * yr.cos() + za * yr.sin()) * d;
    }

    fn tick(&mut self, nivel: &Nivel, f: f32, s: f32, jump: bool) {
        if jump && self.on_ground {
            self.zd = 0.12;
        }
        let accel = if self.on_ground { 0.02 } else { 0.005 };
        self.move_relative(f, s, accel);
        self.zd -= 0.005;
        let (xd, yd, zd) = (self.xd, self.yd, self.zd);
        self.mover(xd, yd, zd, nivel);
        self.xd *= 0.91;
        self.zd *= 0.98;
        self.yd *= 0.91;
        if self.on_ground {
            self.xd *= 0.8;
            self.yd *= 0.8;
        }
    }
}

struct App {
    version: &'static str,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    nivel: Nivel,
    jugador: Jugador,
    zombis: Vec<Zombie>,
    particulas: Vec<Particula>,
    mallas: Vec<(Vec<Vertex>, Vec<u32>)>,
    keys: HashSet<String>,
    grabbed: bool,
    last: Instant,
    tick_acc: f32,
    pintura: u8,
    fps_frames: u32,
    fps_t: Instant,
    fps: u32,
    show_fps: bool,
    hit: Option<((i32, i32, i32), (i32, i32, i32))>,
}

impl App {
    fn new(version: &'static str) -> Self {
        let nivel = Nivel::new(SPAWN_SEED);
        // RubyDung.init: 10 zombis en (128,0,128) con resetPos (posicion al azar
        // en todo el mundo). RD_ZOMBIS_CERCA los pone delante para poder verlos.
        let cerca = std::env::var("RD_ZOMBIS_CERCA").is_ok();
        let mut zombis = Vec::new();
        for i in 0..10 {
            let mut z = Zombie::new(&nivel, 128.0, 0.0, 128.0);
            if cerca {
                z.e.set_pos(128.0 + (i % 5) as f32 * 1.5 - 3.0, 122.0 - (i / 5) as f32 * 2.0, DEPTH as f32 + 4.0);
            } else {
                z.e.reset_pos(&nivel);
            }
            zombis.push(z);
        }
        App {
            version,
            window: None,
            renderer: None,
            nivel,
            jugador: Jugador::new(),
            zombis,
            particulas: Vec::new(),
            mallas: Vec::new(),
            keys: HashSet::new(),
            grabbed: false,
            last: Instant::now(),
            tick_acc: 0.0,
            pintura: tiles::PIEDRA,
            fps_frames: 0,
            fps_t: Instant::now(),
            fps: 0,
            show_fps: false,
            hit: None,
        }
    }

    fn reconstruir_todo(&mut self) {
        let nx = WIDTH / mesh::CHUNK;
        let ny = HEIGHT / mesh::CHUNK;
        let mut mallas = Vec::with_capacity(nx * ny);
        for cy in 0..ny {
            for cx in 0..nx {
                mallas.push(mesh::construir(&self.nivel, cx, cy));
            }
        }
        self.mallas = mallas;
        if let Some(r) = &mut self.renderer {
            r.subir_chunks(self.mallas.clone());
        }
    }

    fn tocar_chunk(&mut self, bx: i32, by: i32) {
        let cx = (bx.max(0) as usize) / mesh::CHUNK;
        let cy = (by.max(0) as usize) / mesh::CHUNK;
        let nx = WIDTH / mesh::CHUNK;
        if cy >= HEIGHT / mesh::CHUNK || cx >= nx {
            return;
        }
        let k = cy * nx + cx;
        if let Some(slot) = self.mallas.get_mut(k) {
            *slot = mesh::construir(&self.nivel, cx, cy);
        }
        let mallas = self.mallas.clone();
        if let Some(r) = &mut self.renderer {
            r.subir_chunks(mallas);
        }
    }

    fn raycast(&self) -> Option<((i32, i32, i32), (i32, i32, i32))> {
        let mut pos = [self.jugador.x, self.jugador.y, self.jugador.z];
        let yaw = self.jugador.y_rot.to_radians();
        let pitch = self.jugador.x_rot.to_radians();
        let (sy, cy) = yaw.sin_cos();
        let (sp, cp) = pitch.sin_cos();
        let mut d = [sy * cp, -cy * cp, sp];
        let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        d = [d[0] / l, d[1] / l, d[2] / l];
        let mut normal = [0i32, 0, 0];
        // El original selecciona con GL (sin limite de distancia: alcanza hasta
        // el far plane). Aqui se marcha hasta 200 bloques.
        let mut t = 0.0f32;
        while t < 200.0 {
            let bx = pos[0].floor() as i32;
            let by = pos[1].floor() as i32;
            let bz = pos[2].floor() as i32;
            if self.nivel.is_solid(bx, by, bz) {
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
                    tmax[i] = if step[i] > 0 {
                        (pos[i].floor() + 1.0 - pos[i]) / d[i]
                    } else {
                        (pos[i] - pos[i].floor()) / -d[i]
                    };
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
                    Window::default_attributes()
                        .with_title(format!("RubyDung RS ({})", self.version))
                        .with_inner_size(winit::dpi::PhysicalSize::new(1024, 768)),
                )
                .unwrap(),
        );
        // Atlas y piel de ESTA version (los tiles no son los de rd-132211)
        std::env::set_var("RD_ATLAS_VERSION", self.version);
        let mut renderer = pollster::block_on(Renderer::new(window.clone()));
        if let Some((w, h, rgba)) = crate::render::cargar_png_del_jar(self.version, "char.png") {
            renderer.pon_piel(w, h, &rgba);
        }
        renderer.mostrar_cruz = true;
        self.renderer = Some(renderer);
        self.window = Some(window);
        self.reconstruir_todo();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(window) = &self.window else { return };
        match event {
            WindowEvent::CloseRequested => {
                let _ = self.nivel.save();
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(r) = &mut self.renderer {
                    r.config.width = size.width.max(1);
                    r.config.height = size.height.max(1);
                    r.surface.configure(&r.device, &r.config);
                }
            }
            WindowEvent::KeyboardInput { event: KeyEvent { logical_key, state, .. }, .. } => {
                if state == ElementState::Pressed {
                    match &logical_key {
                        Key::Named(NamedKey::Escape) => {
                            let _ = self.nivel.save();
                            event_loop.exit();
                        }
                        Key::Named(NamedKey::F3) => self.show_fps = !self.show_fps,
                        // RubyDung: 2 -> piedra, 3 -> tierra, 4 -> ladrillo,
                        // 5 -> madera, 7 -> arbusto (keycodes de LWJGL = tecla)
                        Key::Named(NamedKey::Enter) => {
                            let _ = self.nivel.save();
                        }
                        Key::Character(c) => match c.as_str() {
                            "1" => self.pintura = tiles::PIEDRA,
                            "2" => self.pintura = tiles::TIERRA,
                            "3" => self.pintura = tiles::LADRILLO,
                            "4" => self.pintura = tiles::MADERA,
                            // rd-160052 no define el tile Bush: su jar no lo tiene.
                            "6" if self.version != "rd-160052" => {
                                self.pintura = tiles::ARBUSTO
                            }
                            // RubyDung: G suelta un zombi en la posicion del jugador
                            // (sin resetPos, a diferencia de los 10 iniciales)
                            "g" | "G" => {
                                let z = Zombie::new(&self.nivel, self.jugador.x, self.jugador.y, self.jugador.z);
                                self.zombis.push(z);
                            }
                            _ => {
                                self.keys.insert(c.to_string());
                            }
                        },
                        Key::Named(NamedKey::Space) => {
                            self.keys.insert(" ".to_string());
                        }
                        _ => {}
                    }
                } else if let Key::Character(c) = &logical_key {
                    self.keys.remove(c.as_str());
                } else if let Key::Named(NamedKey::Space) = &logical_key {
                    self.keys.remove(" ");
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
                        match button {
                            // Boton derecho: romper + las 64 particulas del tile
                            MouseButton::Right => {
                                let id = self.nivel.get(bx, by, bz);
                                if self.nivel.set(bx, by, bz, tiles::AIRE) {
                                    particula::destruir(
                                        tiles::textura(id, 0),
                                        bx, by, bz,
                                        &mut self.particulas,
                                    );
                                }
                            }
                            // Boton izquierdo: colocar el bloque elegido
                            MouseButton::Left => {
                                let (nx, ny, nz) = (bx + normal.0, by + normal.1, bz + normal.2);
                                if !self.nivel.is_solid(nx, ny, nz) {
                                    self.nivel.set(nx, ny, nz, self.pintura);
                                }
                            }
                            _ => {}
                        }
                        self.tocar_chunk(bx, by);
                        self.tocar_chunk(bx + normal.0, by + normal.1);
                    }
                }
            }
            _ => {}
        }
    }

    fn device_event(&mut self, _e: &ActiveEventLoop, _d: winit::event::DeviceId, event: DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta } = event {
            if self.grabbed {
                self.jugador.y_rot += delta.0 as f32 * 0.15;
                self.jugador.x_rot -= delta.1 as f32 * 0.15;
                self.jugador.x_rot = self.jugador.x_rot.clamp(-90.0, 90.0);
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
        self.tick_acc += dt;
        let mut steps = (self.tick_acc * 60.0) as i32;
        if steps > 100 { steps = 100; }
        self.tick_acc -= steps as f32 / 60.0;
        for _ in 0..steps {
            self.nivel.tick();
            // particulas
            for p in self.particulas.iter_mut() {
                p.tick(&self.nivel);
            }
            self.particulas.retain(|p| !p.borrada);
            // zombis
            let mut i = 0;
            while i < self.zombis.len() {
                self.zombis[i].tick(&self.nivel);
                if self.zombis[i].e.z < -100.0 {
                    self.zombis.swap_remove(i);
                } else {
                    i += 1;
                }
            }
            self.jugador.tick(&self.nivel, f, s, jump);
        }
        // RubyDung.pick: se recalcula cada frame
        self.hit = self.raycast();

        self.fps_frames += 1;
        if self.fps_t.elapsed().as_millis() >= 500 {
            self.fps = (self.fps_frames as f64 * 1000.0 / self.fps_t.elapsed().as_millis() as f64).round() as u32;
            self.fps_frames = 0;
            self.fps_t = Instant::now();
            if std::env::var("RD_LOG_POS").is_ok() {
                println!("POS {:.3} fondo={:.3} alto={:.3} yaw={:.3} pitch={:.3}",
                    self.jugador.x, self.jugador.y, self.jugador.z,
                    self.jugador.y_rot, self.jugador.x_rot);
            }
        }

        let (sy, cy) = self.jugador.y_rot.to_radians().sin_cos();
        let (sp, cp) = self.jugador.x_rot.to_radians().sin_cos();
        // Ejes de la camara en orden (x, arriba, fondo)
        let fwd = [sy * cp, sp, -cy * cp];
        let der = normalize([-fwd[2], 0.0, fwd[0]]);
        let arr = [
            der[1] * fwd[2] - der[2] * fwd[1],
            der[2] * fwd[0] - der[0] * fwd[2],
            der[0] * fwd[1] - der[1] * fwd[0],
        ];
        let eye = [
            self.jugador.x + fwd[0] * 0.3,
            self.jugador.z + fwd[1] * 0.3,
            self.jugador.y + fwd[2] * 0.3,
        ];

        // Particulas como quads de cara a la camara (malla dinamica)
        let mut pv: Vec<Vertex> = Vec::new();
        let mut pi: Vec<u32> = Vec::new();
        let mut pz: Vec<[f32; 5]> = Vec::new();
        let _ = &mut pz;
        for p in &self.particulas {
            if self.nivel.is_lit(p.x as i32, p.y as i32, p.z as i32) {
                p.vertices(1.0, der, arr, &mut pv, &mut pi);
            }
        }

        // Zombis con el modelo de la piel (solo los que estan a la luz). La
        // posicion se interpola con la fraccion de tick que sobra, como el
        // partialTicks del original.
        let a = (self.tick_acc * 60.0).clamp(0.0, 1.0);
        let mut mv: Vec<[f32; 5]> = Vec::new();
        for z in &self.zombis {
            if self.nivel.is_lit(z.e.x as i32, z.e.y as i32, z.e.z as i32) {
                z.vertices(a, &mut mv);
            }
        }

        let fps = if self.show_fps { Some(self.fps) } else { None };
        let yaw = self.jugador.y_rot.to_radians();
        let pitch = self.jugador.x_rot.to_radians();
        if let (Some(r), Some(w)) = (&mut self.renderer, &self.window) {
            r.set_particulas(&pv);
            // El original siempre enseña el bloque elegido en la esquina
            r.hud_bloque = Some(self.pintura);
            r.render(eye, yaw, pitch, self.hit, fps, &mv);
            w.request_redraw();
        }
    }
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if l < 1e-6 {
        [0.0, 1.0, 0.0]
    } else {
        [v[0] / l, v[1] / l, v[2] / l]
    }
}

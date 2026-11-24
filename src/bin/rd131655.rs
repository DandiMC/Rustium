// rd-131655 (Cave game tech test), la primera version de Minecraft, la que
// Notch grabo en video y nunca publico. Es anterior a rd-132211, asi que el
// terreno no es plano: tiene desniveles y cuevas. Solo existen hierba y roca,
// no se pueden romper ni poner bloques, no hay inventario ni HUD y el mundo no
// se guarda: cada partida genera uno nuevo.
//
// La info sale de minecraft.wiki (Cave game tech test): mundo 256x64x256,
// chunks de 16x16, hierba en las caras con luz (la capa superior de cada
// columna) y roca debajo, cuevas desde la superficie hasta la penultima capa,
// jugador de 1,7 con ojos a 1,62 (aqui se reutiliza el de rd-132211) y R
// mantenida teletransporta a una X/Z al azar a Y=74 en cada tick.

#[path = "../player.rs"]
mod player;
#[path = "../render.rs"]
mod render;
#[path = "../world.rs"]
mod world;

use player::Player;
use render::Renderer;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;
use world::{Level, AIR, DEPTH, GRASS, HEIGHT, WIDTH};
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

const VERSION: &str = "rd-131655";

struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    level: Level,
    player: Player,
    keys: HashSet<String>,
    grabbed: bool,
    last: Instant,
    tick_acc: f32,
    foto: bool,
}

impl App {
    fn new() -> Self {
        let level = mundo_131655();
        let player = Player::new(&level);
        App {
            window: None,
            renderer: None,
            level,
            player,
            keys: HashSet::new(),
            grabbed: false,
            last: Instant::now(),
            tick_acc: 0.0,
            foto: std::env::var_os("RD_FOTO").is_some(),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.foto {
            event_loop.set_control_flow(ControlFlow::Poll);
        }
        let (w, h) = if self.foto {
            (960, 720)
        } else {
            render::tamano_inicial((1024, 768))
        };
        let window = Arc::new(
            event_loop
                .create_window(
                    winit::window::WindowAttributes::default()
                        .with_title(format!("RubyDung RS ({VERSION})"))
                        .with_inner_size(winit::dpi::PhysicalSize::new(w, h)),
                )
                .unwrap(),
        );
        // Las texturas de esta version son las mismas de RubyDung que usa
        // rd-132211; no existe un jar propio de rd-131655.
        std::env::set_var("RD_ATLAS_VERSION", "rd-132211");
        if !self.foto {
            window.focus_window();
        }
        let mut renderer = pollster::block_on(Renderer::new(window.clone()));
        renderer.build_chunks(&self.level);
        self.renderer = Some(renderer);
        // Mouse.setGrabbed(true) del original. En el modo de foto no se
        // secuestra el raton del equipo.
        if !self.foto
            && window
                .set_cursor_grab(winit::window::CursorGrabMode::Locked)
                .is_ok()
        {
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
                let name = match &logical_key {
                    Key::Character(c) => c.to_string(),
                    Key::Named(NamedKey::Space) => " ".to_string(),
                    Key::Named(NamedKey::Super) | Key::Named(NamedKey::Meta) => " ".to_string(),
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
            WindowEvent::MouseInput { state, button: _, .. } => {
                // El original ya tiene el raton capturado al arrancar. No hay
                // clics utiles: en esta version no se rompe ni se coloca nada.
                if state == ElementState::Pressed && !self.grabbed {
                    if window
                        .set_cursor_grab(winit::window::CursorGrabMode::Locked)
                        .is_ok()
                    {
                        window.set_cursor_visible(false);
                        self.grabbed = true;
                    }
                }
            }
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta } = event {
            if self.grabbed {
                self.player.turn(delta.0 as f32, delta.1 as f32);
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let dt = now.duration_since(self.last).as_secs_f32().min(1.0);
        self.last = now;

        let f: f32 = if self.keys.contains("w") { 1.0 } else { 0.0 }
            - if self.keys.contains("s") { 1.0 } else { 0.0 };
        let s: f32 = if self.keys.contains("d") { 1.0 } else { 0.0 }
            - if self.keys.contains("a") { 1.0 } else { 0.0 };
        let jump = self.keys.contains(" ");
        self.tick_acc += dt;
        let mut steps = (self.tick_acc * 60.0) as i32;
        if steps > 100 {
            steps = 100;
        }
        self.tick_acc -= steps as f32 / 60.0;
        let reset = self.keys.contains("r") || self.keys.contains("R");
        for _ in 0..steps {
            // R mantenida: resetPos() en cada tick, con Y = DEPTH + 10 = 74.
            self.player.tick(&self.level, f, s, jump, reset);
        }
        self.player.careo_fijar();

        let a = (self.tick_acc * 60.0).clamp(0.0, 1.0);
        let (ex, ey, ez) = self.player.eye_pos_at(a);
        let yaw = self.player.y_rot.to_radians();
        let pitch = self.player.x_rot.to_radians();
        if let (Some(r), Some(w)) = (&mut self.renderer, &self.window) {
            // Sin seleccion, sin cruz, sin contador de FPS: esta version no
            // tenia nada de eso.
            r.render([ex, ey, ez], yaw, pitch, None, None, &[], &[]);
            w.request_redraw();
        }
    }
}

#[path = "../crash.rs"]
mod crash;
fn main() {
    crash::install();
    // La version no guarda nada; el level.dat de rd-132211 no se toca.
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new();
    event_loop.run_app(&mut app).unwrap();
}

// ----------------------------------------------------------- mundo del tech test

fn idx(x: usize, y: usize, z: usize) -> usize {
    (z * HEIGHT + y) * WIDTH + x
}

fn hash(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374_761_393)
        ^ (y as u32).wrapping_mul(668_265_263)
        ^ (z as u32).wrapping_mul(1_274_126_177)
        ^ seed.wrapping_mul(2_246_822_519);
    h = h.wrapping_mul(3_266_489_917);
    h ^= h >> 13;
    h = h.wrapping_mul(668_265_263);
    h ^= h >> 16;
    (h & 0xFF_FFFF) as f32 / 0xFF_FFFF as f32
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn noise(x: f32, y: f32, z: f32, seed: u32) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let zi = z.floor() as i32;
    let xf = smooth(x - xi as f32);
    let yf = smooth(y - yi as f32);
    let zf = smooth(z - zi as f32);
    let mut v = 0.0;
    for dz in 0..=1 {
        for dy in 0..=1 {
            for dx in 0..=1 {
                let w = (if dx == 0 { 1.0 - xf } else { xf })
                    * (if dy == 0 { 1.0 - yf } else { yf })
                    * (if dz == 0 { 1.0 - zf } else { zf });
                v += w * hash(xi + dx, yi + dy, zi + dz, seed);
            }
        }
    }
    v
}

/// Semilla de la partida. El original elegia un mundo nuevo al azar cada vez;
/// RD_SEED_FIJO=1 deja el mismo mundo para capturas y pruebas.
fn semilla() -> u32 {
    if std::env::var_os("RD_SEED_FIJO").is_some() {
        return 0x1316_55;
    }
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| (d.as_nanos() & 0xFFFF_FFFF) as u32)
        .unwrap_or(0x1316_55)
}

/// Terreno irregular de 7 capas (la superficie cambia de altura) con cuevas
/// que llegan hasta la penultima capa. Solo hay un bloque solido, GRASS, igual
/// que el original: el mallado pinta de hierba la capa mas alta de cada
/// columna y de roca el resto.
fn mundo_131655() -> Level {
    let seed = semilla();
    let mut blocks = vec![AIR; WIDTH * HEIGHT * DEPTH];
    for x in 0..WIDTH {
        for y in 0..HEIGHT {
            // Tres octavas: colinas amplias, terrazas y escalones de un bloque.
            let n1 = noise(x as f32 * 0.035, y as f32 * 0.035, 0.0, seed);
            let n2 = noise(x as f32 * 0.11, y as f32 * 0.11, 0.0, seed ^ 0x9E37);
            let n3 = noise(x as f32 * 0.28, y as f32 * 0.28, 0.0, seed ^ 0x77);
            let mut h = 40
                + ((n1 - 0.5) * 9.0 + (n2 - 0.5) * 3.0 + (n3 - 0.5) * 2.0).round() as i32;
            h = h.clamp(33, 47);
            for z in 0..=h {
                blocks[idx(x, y, z as usize)] = GRASS;
            }
            // Cuevas: dos campos de ruido 3D. El segundo abre salas grandes y
            // el primero serpentea; juntos dejan galerias y entradas al aire.
            // Se incluye la capa h: en el original las cuevas empiezan en la
            // misma superficie y dejan cuencas de roca al descubierto.
            for z in 1..=h {
                let c1 = noise(x as f32 * 0.075, y as f32 * 0.075, z as f32 * 0.09, seed ^ 0x51ED);
                let c2 = noise(x as f32 * 0.030, y as f32 * 0.030, z as f32 * 0.045, seed ^ 0xA53);
                if c1 > 0.56 && c2 > 0.42 {
                    blocks[idx(x, y, z as usize)] = AIR;
                }
            }
        }
    }
    // La capa 0 no se toca: las cuevas llegan hasta la penultima.
    for x in 0..WIDTH {
        for y in 0..HEIGHT {
            blocks[idx(x, y, 0)] = GRASS;
        }
    }
    Level::from_blocks(blocks)
}

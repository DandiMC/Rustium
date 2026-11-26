// Juego de la familia rd-160052 / rd-20090515 / rd-161348 (RubyDung).
//
// Es el mismo codigo para las tres: rd-161348 es el mismo jar que rd-20090515 y
// rd-160052 solo se diferencia en que no define el tile "Bush", que ni el
// generador ni el jugador colocan nunca.
//
// Fiel a RubyDung.java: 10 zombis al empezar, elegir bloque con 1/2/3/4/6,
// G suelta un zombi, Enter guarda level.dat, clic derecho rompe (con 64
// particulas) y el izquierdo coloca el bloque elegido.

use super::level::Nivel;
use super::mesh;
use super::particula::{self, Particula};
use super::tiles;
use crate::entity::{randf, Fisica, Zombie};
use crate::render::{tamano_inicial, Renderer, Vertex};
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
// RubyDung usa Timer(20.0f); cada tick es 1/20 de segundo, no 1/60.
const TPS: f32 = 20.0;
// RD_SMOKE: abre una ventana pequena, coloca un zombi delante de la camara y
// cierra sola tras unos frames. Es el arnes del test de regresion del renderer.
const SMOKE_FRAMES: u32 = 12;

// Chunks que hay que reconstruir al tocar un bloque (o dos: el apuntado y el
// de la cara donde se coloca). Se marca la celda y sus vecinas, como
// LevelRenderer.setDirty, y se recorta al mundo.
fn chunks_tocados(
    ancho: usize,
    alto: usize,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
) -> (usize, usize, usize, usize) {
    let nx = ancho / mesh::CHUNK;
    let ny = alto / mesh::CHUNK;
    (
        ((x0.min(x1) - 1).max(0) as usize / mesh::CHUNK).min(nx - 1),
        ((x0.max(x1) + 1).max(0) as usize / mesh::CHUNK).min(nx - 1),
        ((y0.min(y1) - 1).max(0) as usize / mesh::CHUNK).min(ny - 1),
        ((y0.max(y1) + 1).max(0) as usize / mesh::CHUNK).min(ny - 1),
    )
}

// Teclas de seleccion de bloque de RubyDung: 1=piedra, 2=tierra, 3=ladrillo,
// 4=madera y 6=arbusto. El Java compara Keyboard.getEventKey() con 2,3,4,5,7,
// pero esos son los keycodes de LWJGL de las teclas fisicas 1,2,3,4,6
// (Keyboard.KEY_1=2 ... KEY_4=5, KEY_6=7). rd-160052 no tiene el tile Bush.
fn bloque_para_tecla(version: &str, tecla: &str) -> Option<u8> {
    // 0.0.14a es la primera con las nueve teclas y con menas, arena, grava y
    // madera. La wiki no detalla el orden de la paleta; se sigue el de los ids
    // de Java, que es el que usa el selector de bloque del raton.
    if version == "c0.0.14a" {
        return match tecla {
            "1" => Some(tiles::PIEDRA),
            "2" => Some(tiles::CESPED),
            "3" => Some(tiles::TIERRA),
            "4" => Some(tiles::LADRILLO),
            "5" => Some(tiles::MADERA),
            "6" => Some(tiles::ARENA),
            "7" => Some(tiles::GRAVA),
            "8" => Some(tiles::TRONCO),
            "9" => Some(tiles::HOJAS),
            _ => None,
        };
    }
    match tecla {
        "1" => Some(tiles::PIEDRA),
        "2" => Some(tiles::TIERRA),
        "3" => Some(tiles::LADRILLO),
        "4" => Some(tiles::MADERA),
        "6" if version != "rd-160052" => Some(tiles::ARBUSTO),
        _ => None,
    }
}

pub fn correr(version: &'static str) {
    correr_modo(version, Modo::Legacy);
}

/// c0.0.11a, la primera Classic: mismo motor, pero el mundo sale de LevelGen
/// con NoiseMap, y los clics funcionan como alli (el izquierdo rompe o pinta
/// segun el modo que cambia el derecho).
pub fn correr_clasico(version: &'static str) {
    correr_clasico_con(version, 1024, 768, true);
}

/// c0.0.14a: misma ventana que 0.0.13a (854x480), pero con el atlas nuevo,
/// las nueve teclas de bloque y los bloques que caen.
pub fn correr_clasico_014() {
    tiles::usar_atlas_014();
    correr_clasico_con("c0.0.14a", 854, 480, true);
}

/// Reconstruidas sin jar propio (mc-161607 a mc-161648 y c0.0.3a): comparten
/// motor con 0.0.11a, pero son anteriores al modo de construccion, al contador
/// de fps y al cambio a 640x480 de 0.0.9a. A 1024x768, el izquierdo coloca y
/// el derecho rompe. `escape_libera` separa las mc- (Escape suelta el raton) de
/// c0.0.3a (Escape cierra); `mostrar_version` deja el HUD vacio en las mc-, que
/// no tenian numero de version; `eje_y` es la Y que anadio mc-161616.
pub fn correr_clasico_reconstruida(
    version: &'static str,
    escape_libera: bool,
    mostrar_version: bool,
    eje_y: bool,
) {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new(version, Modo::Clasico);
    app.tamano = tamano_inicial((1024, 768));
    app.escape_libera = escape_libera;
    app.contador = false;
    app.modo_edicion = false;
    app.mostrar_version = mostrar_version;
    app.eje_y = eje_y;
    event_loop.run_app(&mut app).unwrap();
}

/// Reconstruidas sin jar propio: su piel sale de c0.0.11a, la primera Classic
/// que se conserva. Las reuploads 0.0.12a_03/0.0.13a_03 son oficiales y no
/// necesitan este respaldo.
fn es_reconstruida(version: &str) -> bool {
    matches!(
        version,
        "mc-161607"
            | "mc-161616"
            | "mc-161625"
            | "mc-161648"
            | "c0.0.3a"
            | "c0.0.9a"
            | "c0.0.10a"
            | "c0.0.12a"
            | "c0.0.12a_01"
            | "c0.0.12a_02"
            | "c0.0.13a"
    )
}

/// Las versiones que comparten el mundo clasico con liquidos. La 0.0.13a_03
/// usa el mismo generador, pero sin lagos de lava por encima del nivel del mar.
fn es_clasico_con_liquidos(version: &str) -> bool {
    matches!(
        version,
        "c0.0.12a"
            | "c0.0.12a_01"
            | "c0.0.12a_02"
            | "c0.0.12a_03"
            | "c0.0.13a"
            | "c0.0.13a_03"
    )
}

/// 0.0.13a volvio al salto de 0.0.11a y cambio la fisica de agua/lava y los
/// controles de R y salto. Se mantiene separado del resto de Classic para no
/// alterar las variantes de 0.0.12a.
fn es_13a(version: &str) -> bool {
    matches!(version, "c0.0.13a" | "c0.0.13a_03")
}

/// Reconstrucciones (sus jars no se conservan): 0.0.9a y 0.0.10a se montan
/// sobre el codigo de c0.0.11a, que es el primero que queda. La wiki da dos
/// cambios: 0.0.9a corria a 640x480 y Escape cerraba el juego; 0.0.10a anadio
/// que Escape solo suelte el raton.
pub fn correr_clasico_con(
    version: &'static str,
    ancho: u32,
    alto: u32,
    escape_libera: bool,
) {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new(version, Modo::Clasico);
    app.tamano = tamano_inicial((ancho, alto));
    app.escape_libera = escape_libera;
    event_loop.run_app(&mut app).unwrap();
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Modo {
    Legacy,
    Clasico,
}

fn correr_modo(version: &'static str, modo: Modo) {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new(version, modo);
    event_loop.run_app(&mut app).unwrap();
}

struct Jugador {
    ancho: usize,
    alto: usize,
    fondo: usize,
    x: f32,
    y: f32,
    z: f32,
    // Posicion del tick anterior: la camara interpola con la fraccion que sobra
    // (moveCameraToPlayer(a) del original).
    xo: f32,
    yo: f32,
    zo: f32,
    xd: f32,
    yd: f32,
    zd: f32,
    y_rot: f32,
    x_rot: f32,
    on_ground: bool,
    // La caja de colision se guarda, como en Entity del original. Antes se
    // recalculaba desde la altura del ojo en cada tick (z - 1.62) y el redondeo
    // de f32 la dejaba una milesima por debajo del suelo: el clip vertical ya no
    // la reconocia como apoyo y el jugador se hundia un bloque entero a ciertas
    // alturas.
    bb: BlockBox,
}

impl Jugador {
    fn new(ancho: usize, alto: usize, fondo: usize) -> Self {
        // RD_ROT="yaw,pitch" fija la rotacion inicial (comodidad para capturas)
        let (mut y_rot, mut x_rot) = (0.0f32, 0.0f32);
        if let Ok(v) = std::env::var("RD_ROT") {
            let mut it = v.split(',');
            if let (Some(a), Some(b)) = (it.next(), it.next()) {
                y_rot = a.trim().parse().unwrap_or(0.0);
                x_rot = b.trim().parse().unwrap_or(0.0);
            }
        }
        // Como el resetPos del original: punto aleatorio del mapa. RD_SPAWN=f
        // fija la fraccion (careo) y RD_POS="x,fondo,alto" manda del todo.
        // Dos fracciones independientes, como los dos Math.random() del
        // resetPos original (x y fondo no tienen por que coincidir).
        let mut fraccion = || std::env::var("RD_SPAWN").ok()
            .and_then(|v| v.trim().parse::<f32>().ok())
            .map(|v| v.clamp(0.0, 1.0))
            .unwrap_or_else(|| {
                let n = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.subsec_nanos()).unwrap_or(0) as f32;
                n / 1_000_000_000.0
            });
        let (mut px, mut py, mut pz) =
            (ancho as f32 * fraccion(), alto as f32 * fraccion(), fondo as f32 + 10.0);
        if let Ok(v) = std::env::var("RD_POS") {
            let n: Vec<f32> = v.split(',').filter_map(|t| t.trim().parse().ok()).collect();
            if n.len() == 3 {
                px = n[0];
                py = n[1];
                pz = n[2];
            }
        }
        let mut j = Jugador {
            ancho,
            alto,
            fondo,
            x: px,
            y: py,
            z: pz,
            xo: px,
            yo: py,
            zo: pz,
            xd: 0.0, yd: 0.0, zd: 0.0,
            y_rot,
            x_rot,
            on_ground: false,
            bb: BlockBox { x0: 0.0, x1: 0.0, y0: 0.0, y1: 0.0, z0: 0.0, z1: 0.0 },
        };
        j.set_pos(px, py, pz);
        j
    }

    // Entity.setPos: la caja se coloca con la posicion, no se deriva de ella
    // cada tick. Asi el borde superior de un bloque queda exacto al aterrizar.
    fn set_pos(&mut self, x: f32, y: f32, z: f32) {
        self.x = x;
        self.y = y;
        self.z = z;
        self.bb = BlockBox {
            x0: x - 0.3, x1: x + 0.3,
            y0: y - 0.3, y1: y + 0.3,
            z0: z - 1.62, z1: z + 0.18,
        };
    }

    fn bbox(&self) -> BlockBox {
        self.bb
    }

    // Arnes de careo: RD_FIJA=1 fija la camara en cada frame, como el parche
    // del Java original que reescribe player.x/y/z antes de pick/render.
    fn careo_fijar(&mut self) {
        if std::env::var_os("RD_FIJA").is_none() {
            return;
        }
        if let Ok(v) = std::env::var("RD_POS") {
            let n: Vec<f32> = v.split(',').filter_map(|t| t.trim().parse().ok()).collect();
            if n.len() == 3 {
                self.set_pos(n[0], n[1], n[2]);
                self.xo = n[0];
                self.yo = n[1];
                self.zo = n[2];
                self.xd = 0.0;
                self.yd = 0.0;
                self.zd = 0.0;
            }
        }
        if let Ok(v) = std::env::var("RD_ROT") {
            let n: Vec<f32> = v.split(',').filter_map(|t| t.trim().parse().ok()).collect();
            if n.len() == 2 {
                self.y_rot = n[0];
                self.x_rot = n[1];
            }
        }
    }

    fn mover(&mut self, xa: f32, ya: f32, za: f32, nivel: &Nivel) {
        let (xa0, ya0, za0) = (xa, ya, za);
        let area = self.bbox().expanded(xa, ya, za);
        let cubes = nivel.cubes(&area);
        let mut bb = self.bb;

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

        self.bb = bb;
        self.x = (bb.x0 + bb.x1) / 2.0;
        self.y = (bb.y0 + bb.y1) / 2.0;
        self.z = bb.z0 + 1.62;
    }

    // Posicion de la camara con la fraccion de tick `a`, como
    // moveCameraToPlayer(a); el pick usa esta misma posicion (0.3 detras del ojo).
    fn camara(&self, a: f32) -> [f32; 3] {
        [
            self.xo + (self.x - self.xo) * a,
            self.yo + (self.y - self.yo) * a,
            self.zo + (self.z - self.zo) * a,
        ]
    }

    // Entity.resetPos: posicion al azar en todo el mundo y velocidad a cero.
    // Lo usa la R del original en cada tick mientras se mantiene pulsada.
    fn reset_pos(&mut self) {
        self.set_pos(
            randf() * self.ancho as f32,
            randf() * self.alto as f32,
            self.fondo as f32 + 10.0,
        );
        // Entity.resetPos del original solo llama a setPos: conserva la
        // velocidad actual. No la pongas a cero aqui.
    }

    // Liquido que ocupa los pies del jugador. En 0.0.12a el agua y la lava se
    // atraviesan, pero frenan y permiten nadar.
    fn liquido(&self, nivel: &Nivel) -> Option<u8> {
        let x = self.x.floor() as i32;
        let y = self.y.floor() as i32;
        let z = (self.bb.z0 + 0.1).floor() as i32;
        let id = nivel.get(x, y, z);
        if tiles::es_liquido(id) { Some(id) } else { None }
    }

    fn move_relative(&mut self, xa: f32, za: f32, speed: f32) {
        let dist = (xa * xa + za * za).sqrt();
        if dist < 0.01 { return; }
        let d = speed / dist;
        let yr = self.y_rot.to_radians();
        self.xd += (xa * yr.sin() + za * yr.cos()) * d;
        self.yd += (-xa * yr.cos() + za * yr.sin()) * d;
    }

    // Player.tick de rd-160052/rd-20090515/rd-161348 (20 ticks por segundo).
    // Los valores de rd-132211 (0.12/0.005/0.8) no valen aqui: ese build usa
    // Timer(60) y constantes distintas.
    fn tick(&mut self, nivel: &Nivel, f: f32, s: f32, jump: bool, reset: bool) {
        self.tick_con(nivel, f, s, jump, jump, reset, false);
    }

    // `jump_presionado` es el flanco de la tecla y `jump_mantenido` el estado
    // continuo: 0.0.13a consume el salto en tierra pero permite nadar mientras
    // se mantiene pulsado, como el Player.a() original. `reset` tambien es un
    // flanco en 0.0.13a para que R no reinicie la posicion en cada tick.
    fn tick_con(
        &mut self,
        nivel: &Nivel,
        f: f32,
        s: f32,
        jump_presionado: bool,
        jump_mantenido: bool,
        reset: bool,
        es_13a: bool,
    ) {
        self.xo = self.x;
        self.yo = self.y;
        self.zo = self.z;
        if reset {
            self.reset_pos();
        }
        if es_13a {
            // Player.a() de 0.0.13a_03: salto 0.42, +0.04 por tick al nadar,
            // gravedad 0.08/0.02 y friccion 0.8 en agua y 0.5 en lava.
            let liquido = self.liquido(nivel);
            if liquido.is_some() {
                if jump_mantenido {
                    self.zd += 0.04;
                }
            } else if jump_presionado && self.on_ground {
                self.zd = 0.42;
            }
            let accel = if liquido.is_some() { 0.02 } else if self.on_ground { 0.1 } else { 0.02 };
            self.move_relative(f, s, accel);
            let (xd, yd, zd) = (self.xd, self.yd, self.zd);
            self.mover(xd, yd, zd, nivel);
            if liquido.is_some() {
                let roce = if liquido == Some(tiles::LAVA) { 0.5 } else { 0.8 };
                self.xd *= roce;
                self.yd *= roce;
                self.zd *= roce;
                self.zd -= 0.02;
            } else {
                self.xd *= 0.91;
                self.yd *= 0.98;
                self.zd *= 0.91;
                self.zd -= 0.08;
                if self.on_ground {
                    self.xd *= 0.6;
                    self.zd *= 0.6;
                }
            }
            return;
        }
        let en_liquido = self.liquido(nivel).is_some();
        if jump_presionado && (self.on_ground || en_liquido) {
            // Nadar: dentro del agua/lava el salto empuja hacia arriba sin la
            // velocidad del salto en seco.
            self.zd = if en_liquido { 0.18 } else { 0.5 };
        }
        let accel = if en_liquido { 0.05 } else if self.on_ground { 0.1 } else { 0.02 };
        self.move_relative(f, s, accel);
        self.zd -= if en_liquido { 0.02 } else { 0.08 };
        let (xd, yd, zd) = (self.xd, self.yd, self.zd);
        self.mover(xd, yd, zd, nivel);
        let roze = if en_liquido { 0.80 } else { 0.91 };
        self.xd *= roze;
        self.zd *= if en_liquido { 0.80 } else { 0.98 };
        self.yd *= roze;
        if self.on_ground {
            self.xd *= 0.7;
            self.yd *= 0.7;
        }
    }
}

struct App {
    version: &'static str,
    modo: Modo,
    // Classic: 0 rompe (y el velo es liso), 1 pinta el bloque elegido. Lo
    // cambia el boton derecho, como Minecraft.editMode.
    edit_mode: u8,
    // Classic cuenta los chunk updates que enseña en el HUD (Chunk.updates).
    chunk_updates: u32,
    // Ventana de cada build y que hace Escape (antes de 0.0.10a cerraba).
    tamano: (u32, u32),
    escape_libera: bool,
    // 0.0.9a anadio el contador y el modo de construccion; c0.0.3a no los tiene.
    contador: bool,
    modo_edicion: bool,
    // Las mc- son aun mas antiguas: ni contador ni numero de version en pantalla.
    mostrar_version: bool,
    // mc-161616 anadio la Y para invertir el eje del raton.
    eje_y: bool,
    invertir_y: bool,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    nivel: Nivel,
    jugador: Jugador,
    zombis: Vec<Zombie>,
    particulas: Vec<Particula>,
    keys: HashSet<String>,
    grabbed: bool,
    last: Instant,
    tick_acc: f32,
    pintura: u8,
    fps_frames: u32,
    fps_t: Instant,
    fps: u32,
    show_fps: bool,
    // Classic ya enseña los fps en su propio HUD: F3 lo oculta en vez de abrir
    // otro contador. Empieza visible, como el original.
    hud_visible: bool,
    // Extras que el launcher activa por instancia (por defecto, todos menos G).
    extra_coords: bool,
    extra_shots: bool,
    extra_fullscreen: bool,
    g_repeat: bool,
    // 0.0.13a consume estos flancos una sola vez: mantener R ya no reinicia
    // la posicion y mantener salto no hace saltar al aterrizar.
    salto_presionado: bool,
    reinicio_presionado: bool,
    fullscreen: bool,
    pedir_captura: bool,
    hit: Option<((i32, i32, i32), (i32, i32, i32))>,
    smoke_frames: u32,
    // 0.0.14a: mantener el boton repite la accion cada 0,25 s.
    mouse_repeat: Option<winit::event::MouseButton>,
    repeat_acc: f32,
}

impl App {
    fn new(version: &'static str, modo: Modo) -> Self {
        // El original crea sus Random con la hora: el mundo cambia en cada
        // arranque. RD_SEEDS fija los cuatro filtros para el careo y
        // RD_SEED_FIJO conserva la semilla historica del port.
        let seed = if std::env::var_os("RD_SMOKE").is_some()
            || std::env::var_os("RD_SEED_FIJO").is_some()
        {
            SPAWN_SEED
        } else {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as i64)
                .unwrap_or(SPAWN_SEED)
        };
        let mut nivel = match modo {
            Modo::Clasico if version == "c0.0.14a" => Nivel::new_clasico_014(seed),
            Modo::Clasico if version == "c0.0.13a" => Nivel::new_013a_dev(seed),
            Modo::Clasico if version == "c0.0.13a_03" => Nivel::new_clasico_013a03(seed),
            Modo::Clasico if es_clasico_con_liquidos(version) => Nivel::new_clasico_012(seed),
            Modo::Clasico => Nivel::new_clasico(seed),
            Modo::Legacy => Nivel::new(seed),
        };
        // El test necesita una cara de la capa de niebla: un techo sobre la
        // camara deja su columna a oscuras, como una casa del jugador.
        if std::env::var_os("RD_SMOKE").is_some() {
            let cx = (nivel.ancho / 2) as i32;
            let cy = (nivel.alto / 2) as i32;
            for dx in -2..=2 {
                for dy in -2..=2 {
                    nivel.set(cx + dx, cy + dy, 50, tiles::PIEDRA);
                }
            }
        }
        // RubyDung.init: 10 zombis en (128,0,128) con resetPos (posicion al azar
        // en todo el mundo). RD_ZOMBIS_CERCA los pone delante para poder verlos;
        // RD_SMOKE_SIN_ZOMBIS deja la escena limpia para las capturas del velo.
        let cerca = std::env::var("RD_ZOMBIS_CERCA").is_ok();
        // La wiki de 0.0.13a_03 documenta que los mobs ya no aparecen de forma
        // natural. El arnes de humo puede seguir pidiendo uno con RD_SMOKE.
        let sin_zombis = std::env::var_os("RD_SMOKE_SIN_ZOMBIS").is_some()
            || (matches!(version, "c0.0.13a_03" | "c0.0.14a")
                && std::env::var_os("RD_SMOKE").is_none());
        let mut zombis = Vec::new();
        if !sin_zombis {
            let cx = (nivel.ancho / 2) as f32;
            let cy = (nivel.alto / 2) as f32;
            for i in 0..10 {
                let mut z = Zombie::new(&nivel, cx, 0.0, cy);
                if cerca {
                    z.e.set_pos(
                        cx + (i % 5) as f32 * 1.5 - 3.0,
                        cy - 6.0 - (i / 5) as f32 * 2.0,
                        nivel.fondo as f32 + 4.0,
                    );
                } else {
                    z.e.reset_pos(&nivel);
                }
                zombis.push(z);
            }
        }
        let (ancho_mundo, alto_mundo, fondo_mundo) = (nivel.ancho, nivel.alto, nivel.fondo);
        App {
            version,
            modo,
            // RD_SMOKE_PINTAR abre el arnes en modo construir para las capturas
            // del preview de colocacion.
            edit_mode: if std::env::var_os("RD_SMOKE_PINTAR").is_some() { 1 } else { 0 },
            chunk_updates: 0,
            tamano: tamano_inicial((1024, 768)),
            escape_libera: true,
            contador: true,
            modo_edicion: true,
            mostrar_version: true,
            eje_y: false,
            invertir_y: false,
            window: None,
            renderer: None,
            nivel,
            jugador: Jugador::new(ancho_mundo, alto_mundo, fondo_mundo),
            zombis,
            particulas: Vec::new(),
            keys: HashSet::new(),
            grabbed: false,
            last: Instant::now(),
            tick_acc: 0.0,
            pintura: tiles::PIEDRA,
            fps_frames: 0,
            fps_t: Instant::now(),
            fps: 0,
            show_fps: false,
            hud_visible: true,
            extra_coords: std::env::var_os("RD_F3_COORDS").is_some(),
            extra_shots: std::env::var_os("RD_F2_SHOTS").is_some(),
            extra_fullscreen: std::env::var_os("RD_F11_FULLSCREEN").is_some(),
            g_repeat: std::env::var_os("RD_G_REPEAT").is_some(),
            salto_presionado: false,
            reinicio_presionado: false,
            fullscreen: false,
            pedir_captura: false,
            hit: None,
            smoke_frames: 0,
            mouse_repeat: None,
            repeat_acc: 0.0,
        }
    }

    fn actualizar_hud(&mut self) {
        if self.modo != Modo::Clasico {
            return;
        }
        let mut lineas = if self.hud_visible && self.mostrar_version {
            let mut v = vec![(self.version.to_string(), 2.0, 2.0)];
            // 0.0.9a anadio el contador de fps y chunk updates; c0.0.3a es anterior.
            if self.contador {
                v.push((
                    format!("{} fps, {} chunk updates", self.fps, self.chunk_updates),
                    2.0,
                    12.0,
                ));
            }
            v
        } else {
            Vec::new()
        };
        // Extra de instancia: coordenadas junto al HUD, actualizadas por frame.
        if self.hud_visible && self.mostrar_version && self.extra_coords {
            lineas.push((
                format!(
                    "x {:.1} y {:.1} z {:.1}",
                    self.jugador.x, self.jugador.z, self.jugador.y
                ),
                2.0,
                22.0,
            ));
        }
        if let Some(r) = &mut self.renderer {
            r.hud_lineas = lineas;
        }
    }

    fn reconstruir_todo(&mut self) {
        let nx = self.nivel.ancho / mesh::CHUNK;
        let ny = self.nivel.alto / mesh::CHUNK;
        let mut capa0 = Vec::with_capacity(nx * ny);
        let mut capa1 = Vec::with_capacity(nx * ny);
        let mut liquidos0 = Vec::with_capacity(nx * ny);
        let mut liquidos1 = Vec::with_capacity(nx * ny);
        for cy in 0..ny {
            for cx in 0..nx {
                capa0.push(mesh::construir(&self.nivel, cx, cy, 0));
                capa1.push(mesh::construir(&self.nivel, cx, cy, 1));
                liquidos0.push(mesh::construir_liquidos(&self.nivel, cx, cy, 0));
                liquidos1.push(mesh::construir_liquidos(&self.nivel, cx, cy, 1));
            }
        }
        if self.modo == Modo::Clasico {
            // Chunk.rebuild actualiza las dos capas y suma dos updates.
            self.chunk_updates += (2 * nx * ny) as u32;
        }
        if let Some(r) = &mut self.renderer {
            r.subir_chunks(capa0, capa1);
            r.subir_liquidos(liquidos0, liquidos1);
        }
    }

    // Consume los cambios que Level avisaba a LevelRenderer. Incluye tanto los
    // clics como el tick del nivel (hierba/arbustos); sin esto la malla se
    // quedaba con el bloque viejo hasta el siguiente clic.
    fn reconstruir_cambios(&mut self) {
        if self.renderer.is_none() {
            return;
        }
        for (x, y) in self.nivel.tomar_cambios() {
            self.tocar_zona(x, y, x, y);
        }
    }

    // LevelRenderer.setDirty del original: un bloque tocado marca las celdas de
    // alrededor (x-1..x+1, y-1..y+1). Cambiar la columna de luz altera caras que
    // pueden estar en un chunk vecino; antes solo se reconstruia el chunk del
    // bloque y quedaban caras viejas en las costuras de 16.
    fn tocar_zona(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        let nx = self.nivel.ancho / mesh::CHUNK;
        let (cx0, cx1, cy0, cy1) =
            chunks_tocados(self.nivel.ancho, self.nivel.alto, x0, y0, x1, y1);
        let nivel = &self.nivel;
        let Some(r) = &mut self.renderer else { return };
        let mut n = 0u32;
        for cy in cy0..=cy1 {
            for cx in cx0..=cx1 {
                let k = cy * nx + cx;
                r.actualizar_chunk(
                    k,
                    mesh::construir(nivel, cx, cy, 0),
                    mesh::construir(nivel, cx, cy, 1),
                );
                r.actualizar_liquidos(
                    k,
                    mesh::construir_liquidos(nivel, cx, cy, 0),
                    mesh::construir_liquidos(nivel, cx, cy, 1),
                );
                n += 2;
            }
        }
        if self.modo == Modo::Clasico {
            self.chunk_updates += n;
        }
    }

    // Minecraft.isFree: no se pinta encima de un bloque que ocupe el jugador o
    // un zombi. La caja del bloque nuevo es la unidad entera.
    fn casilla_libre(&self, x: i32, y: i32, z: i32) -> bool {
        let caja = BlockBox {
            x0: x as f32,
            y0: y as f32,
            z0: z as f32,
            x1: x as f32 + 1.0,
            y1: y as f32 + 1.0,
            z1: z as f32 + 1.0,
        };
        let toca = |b: &BlockBox| {
            b.x1 > caja.x0 && b.x0 < caja.x1
                && b.y1 > caja.y0 && b.y0 < caja.y1
                && b.z1 > caja.z0 && b.z0 < caja.z1
        };
        if toca(&self.jugador.bb) {
            return false;
        }
        !self.zombis.iter().any(|z| toca(&z.e.bb))
    }

    fn raycast(&self, camara: [f32; 3]) -> Option<((i32, i32, i32), (i32, i32, i32))> {
        let (ex, ey, ez) = (camara[0], camara[1], camara[2]);
        let yaw = self.jugador.y_rot.to_radians();
        let pitch = self.jugador.x_rot.to_radians();
        let (sy, cy) = yaw.sin_cos();
        let (sp, cp) = pitch.sin_cos();
        let mut d = [sy * cp, -cy * cp, sp];
        let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        d = [d[0] / l, d[1] / l, d[2] / l];
        // La camara del original va 0.3 detras del ojo y pick() solo prueba
        // player.bb.grow(3).
        let mut pos = [ex - d[0] * 0.3, ey - d[1] * 0.3, ez - d[2] * 0.3];
        let limit = self.jugador.bbox().grown(3.0, 3.0, 3.0);
        let mut normal = [0i32, 0, 0];
        let mut t = 0.0f32;
        while t < 200.0 {
            if pos[0] < limit.x0 || pos[0] > limit.x1
                || pos[1] < limit.y0 || pos[1] > limit.y1
                || pos[2] < limit.z0 || pos[2] > limit.z1
            {
                return None;
            }
            let bx = pos[0].floor() as i32;
            let by = pos[1].floor() as i32;
            let bz = pos[2].floor() as i32;
            // LevelRenderer.pick prueba todos los tiles no vacios, no solo los
            // solidos: el arbusto (isSolid=false) tambien se apunta y se rompe.
            if self.nivel.get(bx, by, bz) != tiles::AIRE {
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

impl App {
    /// 0.0.14a: mantener el izquierdo construye o rompe cada 0,25 s, como el
    /// original. Solo se repite el izquierdo; el derecho cambia de modo.
    fn repetir_clic(&mut self) {
        let a = (self.tick_acc * TPS).clamp(0.0, 1.0);
        if let Some(((bx, by, bz), normal)) = self.raycast(self.jugador.camara(a)) {
            if self.edit_mode == 0 {
                let id = self.nivel.get(bx, by, bz);
                if self.nivel.set(bx, by, bz, tiles::AIRE) {
                    particula::destruir(
                        tiles::textura_particula(id),
                        bx, by, bz,
                        &mut self.particulas,
                    );
                }
            } else {
                let (nx, ny, nz) = (bx + normal.0, by + normal.1, bz + normal.2);
                if self.casilla_libre(nx, ny, nz) {
                    self.nivel.set(nx, ny, nz, self.pintura);
                }
            }
        }
        self.reconstruir_cambios();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let smoke = std::env::var_os("RD_SMOKE").is_some();
        // El test no depende de que el escritorio mande eventos de pintado.
        if smoke {
            event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
        }
        let (w, h) = if smoke { (480, 360) } else { self.tamano };
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title(format!("RubyDung RS ({})", self.version))
                        .with_inner_size(winit::dpi::PhysicalSize::new(w, h)),
                )
                .unwrap(),
        );
        // Atlas y piel de ESTA version (los tiles no son los de rd-132211)
        std::env::set_var("RD_ATLAS_VERSION", self.version);
        let mut renderer = pollster::block_on(Renderer::new(window.clone()));
        if !smoke {
            window.focus_window();
        }
        // Las reconstruidas no tienen jar propio: su piel es la de c0.0.11a,
        // igual que el atlas. Sin ella el renderer no dibuja a los humanos.
        let piel = crate::render::cargar_png_del_jar(self.version, "char.png").or_else(|| {
            if es_reconstruida(self.version) {
                crate::render::cargar_png_del_jar("c0.0.11a", "char.png")
            } else {
                None
            }
        });
        if let Some((w, h, rgba)) = piel {
            renderer.pon_piel(w, h, &rgba);
        }
        renderer.mostrar_cruz = true;
        // setupFog(1) de esta familia: GL_EXP densidad 0.06 con el color casi
        // negro para las caras a oscuras; su renderHit usa medio alfa.
        renderer.fog_densidad = 0.06;
        renderer.hit_alpha_escala = 0.5;
        renderer.hit_preview = None;
        self.renderer = Some(renderer);
        // Mouse.setGrabbed(true) del original: el primer clic debe romper o
        // colocar, no quedarse solo en capturar el cursor.
        // El test no puede secuestrar el raton del equipo que lo ejecuta.
        if !smoke && window.set_cursor_grab(winit::window::CursorGrabMode::Locked).is_ok() {
            window.set_cursor_visible(false);
            self.grabbed = true;
        }
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
            WindowEvent::KeyboardInput { event: KeyEvent { logical_key, state, repeat, .. }, .. } => {
                // Las teclas autorepetidas no deben disparar acciones: mantener G
                // generaba un zombi por cada repeticion en vez de uno por pulsacion.
                // Solo G puede repetir, y unicamente si la instancia lo pide.
                // Windows repite una tecla mantenida con state=Pressed y
                // repeat=true. Esas repeticiones no pueden caer en la rama de
                // soltar: borraban "w" del conjunto medio segundo despues de
                // empezar a andar y el jugador se paraba solo.
                if state == ElementState::Released {
                    match &logical_key {
                        Key::Character(c) => {
                            self.keys.remove(c.to_ascii_lowercase().as_str());
                        }
                        Key::Named(NamedKey::Space)
                        | Key::Named(NamedKey::Super)
                        | Key::Named(NamedKey::Meta) => {
                            self.keys.remove(" ");
                        }
                        Key::Named(NamedKey::ArrowUp) => { self.keys.remove("arrowup"); }
                        Key::Named(NamedKey::ArrowDown) => { self.keys.remove("arrowdown"); }
                        Key::Named(NamedKey::ArrowLeft) => { self.keys.remove("arrowleft"); }
                        Key::Named(NamedKey::ArrowRight) => { self.keys.remove("arrowright"); }
                        _ => {}
                    }
                    return;
                }
                let g_repite = self.g_repeat
                    && matches!(&logical_key, Key::Character(c) if c.eq_ignore_ascii_case("g"));
                if state == ElementState::Pressed && (!repeat || g_repite) {
                    match &logical_key {
                        Key::Named(NamedKey::Escape) => {
                            if self.escape_libera {
                                // 0.0.10a: Escape suelta el raton en vez de cerrar.
                                let _ = window.set_cursor_grab(winit::window::CursorGrabMode::None);
                                window.set_cursor_visible(true);
                                self.grabbed = false;
                            } else {
                                let _ = self.nivel.save();
                                event_loop.exit();
                            }
                        }
                        Key::Named(NamedKey::F3) => {
                            if self.modo == Modo::Clasico {
                                // Classic: F3 oculta o muestra su HUD (version y
                                // "N fps, N chunk updates"), que ya viene visible.
                                self.hud_visible = !self.hud_visible;
                                self.actualizar_hud();
                            } else {
                                self.show_fps = !self.show_fps;
                            }
                        }
                        Key::Named(NamedKey::F2) => {
                            if self.extra_shots {
                                self.pedir_captura = true;
                            }
                        }
                        Key::Named(NamedKey::F11) => {
                            if self.extra_fullscreen {
                                self.fullscreen = !self.fullscreen;
                                window.set_fullscreen(if self.fullscreen {
                                    Some(winit::window::Fullscreen::Borderless(None))
                                } else {
                                    None
                                });
                            }
                        }
                        // RubyDung: 1 -> piedra, 2 -> tierra, 3 -> ladrillo,
                        // 4 -> madera, 6 -> arbusto. El Java usa los keycodes
                        // 2/3/4/5/7, que en LWJGL son esas teclas fisicas.
                        Key::Named(NamedKey::Enter) => {
                            let _ = self.nivel.save();
                        }
                        Key::Character(c) => {
                            if c.eq_ignore_ascii_case("r") && es_13a(self.version) {
                                self.reinicio_presionado = true;
                            }
                            if let Some(id) = bloque_para_tecla(self.version, c.as_str()) {
                                self.pintura = id;
                            } else if c.eq_ignore_ascii_case("y") && self.eje_y {
                                // mc-161616: Y invierte el eje del raton.
                                self.invertir_y = !self.invertir_y;
                            } else if c.eq_ignore_ascii_case("g") && self.version != "rd-160052" {
                                // RubyDung: G suelta un zombi en la posicion del jugador
                                // (sin resetPos, a diferencia de los 10 iniciales).
                                // La wiki data la tecla en rd-20090515, asi que
                                // rd-160052, el build anterior, no la tiene.
                                let mut z = Zombie::new(&self.nivel, self.jugador.x, self.jugador.y, self.jugador.z);
                                // El constructor del Java legacy llama a setPos, que
                                // recoloca tambien la caja (bb). El constructor
                                // compartido solo cambia x/y/z, asi que sin esto el
                                // zombi de G conservaba la caja del resetPos aleatorio
                                // y saltaba a otro punto en el primer tick.
                                z.e.set_pos(self.jugador.x, self.jugador.y, self.jugador.z);
                                self.zombis.push(z);
                            } else {
                                // En minusculas: con Mayusculas pulsadas el
                                // movimiento tambien tiene que seguir.
                                self.keys.insert(c.to_ascii_lowercase());
                            }
                        }
                        // La tecla Windows/Meta tambien salta (keycode 219).
                        Key::Named(NamedKey::Space)
                        | Key::Named(NamedKey::Super)
                        | Key::Named(NamedKey::Meta) => {
                            self.keys.insert(" ".to_string());
                            if es_13a(self.version) {
                                self.salto_presionado = true;
                            }
                        }
                        Key::Named(NamedKey::ArrowUp) => {
                            self.keys.insert("arrowup".to_string());
                        }
                        Key::Named(NamedKey::ArrowDown) => {
                            self.keys.insert("arrowdown".to_string());
                        }
                        Key::Named(NamedKey::ArrowLeft) => {
                            self.keys.insert("arrowleft".to_string());
                        }
                        Key::Named(NamedKey::ArrowRight) => {
                            self.keys.insert("arrowright".to_string());
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if state == ElementState::Released && self.mouse_repeat == Some(button) {
                    self.mouse_repeat = None;
                }
                if state == ElementState::Pressed {
                    if !self.grabbed {
                        if window
                            .set_cursor_grab(winit::window::CursorGrabMode::Locked)
                            .is_ok()
                        {
                            window.set_cursor_visible(false);
                            self.grabbed = true;
                        }
                        // El original arranca con el raton capturado: el primer
                        // clic tambien rompe o coloca.
                    }
                    let a = (self.tick_acc * TPS).clamp(0.0, 1.0);
                    // 0.0.14a: el boton central copia el bloque apuntado si esta
                    // en la paleta de las teclas 1-9.
                    if button == MouseButton::Middle && self.modo == Modo::Clasico {
                        if let Some(((bx, by, bz), _)) = self.raycast(self.jugador.camara(a)) {
                            let id = self.nivel.get(bx, by, bz);
                            if tiles::pintable_014(id) {
                                self.pintura = id;
                            }
                        }
                    }
                    if self.modo == Modo::Clasico
                        && self.version == "c0.0.14a"
                        && button == MouseButton::Left
                    {
                        self.mouse_repeat = Some(button);
                        self.repeat_acc = 0.0;
                    }
                    if self.modo == Modo::Clasico && !self.modo_edicion {
                        // c0.0.3a, antes del modo que anadio 0.0.9a: el
                        // izquierdo coloca (aun dentro del jugador, como el
                        // fallo que 0.0.9a arreglo) y el derecho rompe.
                        if let Some(((bx, by, bz), normal)) = self.raycast(self.jugador.camara(a)) {
                            match button {
                                MouseButton::Left => {
                                    let (nx, ny, nz) = (bx + normal.0, by + normal.1, bz + normal.2);
                                    self.nivel.set(nx, ny, nz, self.pintura);
                                }
                                MouseButton::Right => {
                                    let id = self.nivel.get(bx, by, bz);
                                    if self.nivel.set(bx, by, bz, tiles::AIRE) {
                                        particula::destruir(
                                            tiles::textura_particula(id),
                                            bx, by, bz,
                                            &mut self.particulas,
                                        );
                                    }
                                }
                                _ => {}
                            }
                        }
                        self.reconstruir_cambios();
                    } else if self.modo == Modo::Clasico {
                        // Minecraft.handleMouseClick: el izquierdo rompe o
                        // pinta segun editMode y el derecho alterna el modo.
                        match button {
                            MouseButton::Right => {
                                self.edit_mode = (self.edit_mode + 1) % 2;
                            }
                            MouseButton::Left => {
                                if let Some(((bx, by, bz), normal)) = self.raycast(self.jugador.camara(a)) {
                                    if self.edit_mode == 0 {
                                        let id = self.nivel.get(bx, by, bz);
                                        if self.nivel.set(bx, by, bz, tiles::AIRE) {
                                            particula::destruir(
                                                tiles::textura_particula(id),
                                                bx, by, bz,
                                                &mut self.particulas,
                                            );
                                        }
                                    } else {
                                        let (nx, ny, nz) = (bx + normal.0, by + normal.1, bz + normal.2);
                                        if self.casilla_libre(nx, ny, nz) {
                                            self.nivel.set(nx, ny, nz, self.pintura);
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                        self.reconstruir_cambios();
                    } else if let Some(((bx, by, bz), normal)) = self.raycast(self.jugador.camara(a)) {
                        match button {
                            // Boton derecho: romper + las 64 particulas del tile
                            MouseButton::Right => {
                                let id = self.nivel.get(bx, by, bz);
                                if self.nivel.set(bx, by, bz, tiles::AIRE) {
                                    particula::destruir(
                                        tiles::textura_particula(id),
                                        bx, by, bz,
                                        &mut self.particulas,
                                    );
                                }
                            }
                            // Boton izquierdo: colocar el bloque elegido
                            MouseButton::Left => {
                                let (nx, ny, nz) = (bx + normal.0, by + normal.1, bz + normal.2);
                                // RubyDung no comprueba solidez al colocar: el
                                // setTile de la cara contigua sustituye lo que
                                // hubiera (tambien un arbusto).
                                self.nivel.set(nx, ny, nz, self.pintura);
                            }
                            _ => {}
                        }
                        // El set de arriba ya anoto la columna en Nivel; ahora
                        // se reconstruyen los chunks afectados.
                        self.reconstruir_cambios();
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
                let dy = delta.1 as f32 * 0.15;
                self.jugador.x_rot -= if self.invertir_y { -dy } else { dy };
                self.jugador.x_rot = self.jugador.x_rot.clamp(-90.0, 90.0);
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let dt = now.duration_since(self.last).as_secs_f32().min(1.0);
        self.last = now;
        if self.version == "c0.0.14a" {
            if self.mouse_repeat.is_some() {
                self.repeat_acc += dt;
                if self.repeat_acc >= 0.25 {
                    self.repeat_acc -= 0.25;
                    self.repetir_clic();
                }
            } else {
                self.repeat_acc = 0.0;
            }
        }

        let f: f32 = if self.keys.contains("w") || self.keys.contains("arrowup") { 1.0 } else { 0.0 }
            - if self.keys.contains("s") || self.keys.contains("arrowdown") { 1.0 } else { 0.0 };
        let s: f32 = if self.keys.contains("d") || self.keys.contains("arrowright") { 1.0 } else { 0.0 }
            - if self.keys.contains("a") || self.keys.contains("arrowleft") { 1.0 } else { 0.0 };
        let jump = self.keys.contains(" ");
        let reset_mantenido = self.keys.contains("r") || self.keys.contains("R");
        let version_13a = es_13a(self.version);
        self.tick_acc += dt;
        let mut steps = (self.tick_acc * TPS) as i32;
        if steps > 100 { steps = 100; }
        self.tick_acc -= steps as f32 / TPS;
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
                self.zombis[i].tick_con(&self.nivel, Fisica::ZOMBI_LEGACY);
                if self.zombis[i].e.z < -100.0 {
                    self.zombis.swap_remove(i);
                } else {
                    i += 1;
                }
            }
            if version_13a {
                self.jugador.tick_con(
                    &self.nivel,
                    f,
                    s,
                    self.salto_presionado,
                    jump,
                    self.reinicio_presionado,
                    true,
                );
                self.salto_presionado = false;
                self.reinicio_presionado = false;
            } else {
                self.jugador.tick(
                    &self.nivel,
                    f,
                    s,
                    jump,
                    reset_mantenido,
                );
            }
        }
        // LevelRenderer.updateDirtyChunks: una vez por frame, despues de todos
        // los ticks, para que la hierba/arbustos cambiados por el nivel se vean.
        self.reconstruir_cambios();
        // Careo: la camara fija manda antes del pick (igual que el parche del
        // original, que reescribe player.x/y/z dentro de render antes de pick).
        self.jugador.careo_fijar();
        // RubyDung.pick: se recalcula cada frame con la camara interpolada.
        let a = (self.tick_acc * TPS).clamp(0.0, 1.0);
        let camara = self.jugador.camara(a);
        self.hit = self.raycast(camara);

        self.fps_frames += 1;
        if self.fps_t.elapsed().as_millis() >= 500 {
            self.fps = (self.fps_frames as f64 * 1000.0 / self.fps_t.elapsed().as_millis() as f64).round() as u32;
            self.fps_frames = 0;
            self.fps_t = Instant::now();
            if self.modo == Modo::Clasico {
                // Minecraft.run: "N fps, N chunk updates" y se pone a cero el
                // contador cada segundo. La version va encima, en (2,2).
                self.actualizar_hud();
                self.chunk_updates = 0;
            }
            if std::env::var("RD_LOG_POS").is_ok() {
                println!("POS {:.3} fondo={:.3} alto={:.3} yaw={:.3} pitch={:.3}",
                    self.jugador.x, self.jugador.y, self.jugador.z,
                    self.jugador.y_rot, self.jugador.x_rot);
            }
        }

        let (sy, cy) = self.jugador.y_rot.to_radians().sin_cos();
        let (sp, cp) = self.jugador.x_rot.to_radians().sin_cos();
        // Ejes de la camara en orden (x, arriba, fondo). La derecha depende solo
        // del yaw; con la proyeccion de fwd se degeneraba al mirar a 90 grados.
        let fwd = [sy * cp, sp, -cy * cp];
        let der = [cy, 0.0, sy];
        let arr = [
            der[1] * fwd[2] - der[2] * fwd[1],
            der[2] * fwd[0] - der[0] * fwd[2],
            der[0] * fwd[1] - der[1] * fwd[0],
        ];
        // El renderer ya resta 0.3 detras del ojo (glTranslatef(0,0,-0.3) del
        // original); no hay que adelantarlo aqui. La posicion va interpolada
        // entre el tick anterior y el actual, como moveCameraToPlayer(a).
        let eye = [camara[0], camara[2], camara[1]];

        // Particulas como quads de cara a la camara (malla dinamica)
        let mut pv: Vec<Vertex> = Vec::new();
        let mut pi: Vec<u32> = Vec::new();
        let mut pv_fondo: Vec<Vertex> = Vec::new();
        let mut pi_fondo: Vec<u32> = Vec::new();
        for p in &self.particulas {
            if self.nivel.is_lit(p.x as i32, p.y as i32, p.z as i32) {
                p.vertices(a, der, arr, &mut pv, &mut pi);
            } else {
                p.vertices(a, der, arr, &mut pv_fondo, &mut pi_fondo);
            }
        }

        // Zombis con el modelo de la piel, separados por luz como el original:
        // los claros van antes de la capa 1 y los oscuros dentro de su niebla.
        // La posicion se interpola con la fraccion de tick que sobra, como el
        // partialTicks del original.
        let smoke = std::env::var_os("RD_SMOKE").is_some();
        if smoke {
            // Un zombi a tres bloques de la camara, siempre en la lista clara:
            // asi el test recorre el camino que fallaba (modelo -> niebla).
            if let Some(z0) = self.zombis.first_mut() {
                let (x, y, z) = (
                    camara[0] + sy * cp * 3.0,
                    camara[1] - cy * cp * 3.0,
                    camara[2] + sp * 3.0 - 1.5,
                );
                z0.e.set_pos(x, y, z);
                z0.e.xo = x;
                z0.e.yo = y;
                z0.e.zo = z;
            }
        }
        let mut mv: Vec<[f32; 5]> = Vec::new();
        let mut mv_fondo: Vec<[f32; 5]> = Vec::new();
        for (i, z) in self.zombis.iter().enumerate() {
            if smoke && i == 0 {
                z.vertices(a, &mut mv);
            } else if self.nivel.is_lit(z.e.x as i32, z.e.y as i32, z.e.z as i32) {
                z.vertices(a, &mut mv);
            } else {
                z.vertices(a, &mut mv_fondo);
            }
        }

        // Classic renderHit: en modo pintar el velo va en la casilla donde
        // caeria el bloque, no en la apuntada.
        // renderHit con mode 1: el preview va en la casilla contigua y dibuja
        // el bloque elegido, no el velo blanco del modo romper.
        let mut hit = self.hit;
        let mut preview = None;
        if self.modo == Modo::Clasico && self.modo_edicion && self.edit_mode == 1 {
            preview = Some(self.pintura);
            if let Some(((x, y, z), n)) = hit {
                hit = Some(((x + n.0, y + n.1, z + n.2), n));
            }
        }
        let fps = if self.show_fps { Some(self.fps) } else { None };
        let yaw = self.jugador.y_rot.to_radians();
        let pitch = self.jugador.x_rot.to_radians();
        // El HUD de Classic se refresca cada frame: las coordenadas del extra
        // tienen que seguir al jugador, no esperar al contador de medio segundo.
        if self.modo == Modo::Clasico {
            self.actualizar_hud();
        }
        if let (Some(r), Some(w)) = (&mut self.renderer, &self.window) {
            r.set_particulas(&pv, &pv_fondo);
            // 0.0.14a base: plano de nubes a nivel.alto+2 (Level.depth+2 en el
            // original, donde depth es la altura). La extension del plano usa
            // el fondo del mundo. _08 no cambio las nubes.
            r.nubes = self.version == "c0.0.14a";
            r.nubes_dim = [
                self.nivel.ancho as f32,
                self.nivel.fondo as f32,
                self.nivel.alto as f32,
            ];
            // 0.0.13a_03 elige GL_EXP: agua 0.02/0.02/0.2 con densidad 0.1,
            // lava 0.6/0.1/0 con densidad 2.0. Antes de 13a la niebla de lava
            // era naranja y menos densa; el resto de Classic conserva la niebla
            // oscura normal del port.
            // 0.0.14a base: niebla lineal azul 0->1024 en el pase iluminado
            // (K por defecto del original, c=0) y exponencial 0.01 con
            // (14,11,10) en el oscuro. Bajo agua/lava usa los mismos
            // colores/densidades que 0.0.13a, como setupFog del original.
            let es_014 = self.version == "c0.0.14a";
            if es_13a(self.version) || es_014 {
                match self.jugador.liquido(&self.nivel) {
                    Some(tiles::AGUA) => {
                        r.fog_color = [0.02, 0.02, 0.2, 1.0];
                        r.fog_densidad = 0.1;
                        r.fog_densidad_principal = 0.1;
                        r.fog_color_principal = r.fog_color;
                        r.fog_exp_principal = 1.0;
                        r.fog_linear_principal = 0.0;
                    }
                    Some(tiles::LAVA) => {
                        r.fog_color = [0.6, 0.1, 0.0, 1.0];
                        r.fog_densidad = 2.0;
                        r.fog_densidad_principal = 2.0;
                        r.fog_color_principal = r.fog_color;
                        r.fog_exp_principal = 1.0;
                        r.fog_linear_principal = 0.0;
                    }
                    _ => {
                        r.fog_color = [14.0 / 255.0, 11.0 / 255.0, 10.0 / 255.0, 1.0];
                        r.fog_densidad = if es_014 { 0.01 } else { 0.06 };
                        r.fog_densidad_principal = 0.0;
                        if es_014 {
                            r.fog_color_principal = [0.5, 0.8, 1.0, 1.0];
                            r.fog_exp_principal = 0.0;
                            r.fog_linear_principal = 1.0;
                            r.fog_start_principal = 0.0;
                            r.fog_end_principal = 1024.0;
                        } else {
                            r.fog_color_principal = r.fog_color;
                            r.fog_exp_principal = 1.0;
                            r.fog_linear_principal = 0.0;
                        }
                    }
                }
            } else {
                r.fog_color = [14.0 / 255.0, 11.0 / 255.0, 10.0 / 255.0, 1.0];
                r.fog_densidad = 0.06;
                r.fog_densidad_principal = 0.0;
                r.fog_color_principal = r.fog_color;
                r.fog_exp_principal = 1.0;
                r.fog_linear_principal = 0.0;
            }
            r.hit_preview = preview;
            // Extras de instancia: coordenadas en las versiones sin HUD propio
            // (F3) y captura de pantalla (F2).
            r.coords = if self.modo != Modo::Clasico && self.show_fps && self.extra_coords {
                Some(format!(
                    "x {:.1} y {:.1} z {:.1}",
                    self.jugador.x, self.jugador.z, self.jugador.y
                ))
            } else {
                None
            };
            r.pedir_captura = self.pedir_captura;
            self.pedir_captura = false;
            // RD_SMOKE_CAPTURA guarda un frame del arnes en screenshots/ para
            // comparar el resaltado con y sin RD_HIT_BIAS (reproduccion del
            // z-fighting). Misma camara y mismo frame en las dos capturas.
            if smoke && std::env::var_os("RD_SMOKE_CAPTURA").is_some() && self.smoke_frames == 4
            {
                r.pedir_captura = true;
            }
            // El original siempre enseña el bloque elegido en la esquina
            r.hud_bloque = if self.mostrar_version { Some(self.pintura) } else { None };
            r.render(eye, yaw, pitch, hit, fps, &mv, &mv_fondo);
            w.request_redraw();
            if smoke {
                self.smoke_frames += 1;
                if self.smoke_frames >= SMOKE_FRAMES {
                    let niebla = r.chunks_fog.iter().flatten().filter(|c| c.icount > 0).count();
                    println!("SMOKE OK lit={} fog={}", mv.len(), niebla);
                    event_loop.exit();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_bloque_en_la_costura_marca_los_chunks_vecinos() {
        // (15,15) esta en la esquina del chunk 0: la luz cambia caras de los
        // chunks 0, 1 y 4, asi que hay que reconstruir los cuatro de la esquina.
        assert_eq!(chunks_tocados(256, 256, 15, 15, 15, 15), (0, 1, 0, 1));
        // Un bloque en el centro solo toca su chunk.
        assert_eq!(chunks_tocados(256, 256, 8, 8, 8, 8), (0, 0, 0, 0));
        // Fuera del mundo se recorta al ultimo chunk.
        assert_eq!(chunks_tocados(256, 256, 300, 300, 300, 300), (15, 15, 15, 15));
    }

    #[test]
    fn reset_pos_conserva_la_velocidad_como_el_original() {
        let mut j = Jugador::new(256, 256, 64);
        j.xd = 1.0;
        j.yd = 2.0;
        j.zd = 3.0;
        j.reset_pos();
        assert_eq!((j.xd, j.yd, j.zd), (1.0, 2.0, 3.0));
    }

    #[test]
    fn el_013a_usa_el_salto_corto_de_la_0_0_11a() {
        let nivel = Nivel::new(12345);
        let mut j = Jugador::new(256, 256, 64);
        j.set_pos(128.5, 128.5, 100.0);
        j.on_ground = true;
        // 0.0.13a_03: zd = 0.42 y despues 0.91 de roce menos 0.08 de gravedad.
        j.tick_con(&nivel, 0.0, 0.0, true, false, false, true);
        assert!(j.zd > 0.0 && j.zd < 0.35, "salto 13a inesperado: {}", j.zd);
    }

    #[test]
    fn el_jugador_no_atraviesa_suelos_a_ninguna_altura() {
        for altura in [18, 24, 30, 36, 42, 48, 52, 56, 60] {
            let mut nivel = Nivel::new(12345);
            // Una plataforma ancha y despejada: el jugador cae y luego camina
            // por encima, sin colinas naturales de por medio.
            for x in 64..192 {
                for y in 64..192 {
                    for z in 0..nivel.fondo {
                        let id = if z <= altura { tiles::PIEDRA } else { tiles::AIRE };
                        nivel.set(x as i32, y as i32, z as i32, id);
                    }
                }
            }
            let mut j = Jugador::new(256, 256, 64);
            j.set_pos(100.5, 170.5, 63.0);
            j.xo = j.x;
            j.yo = j.y;
            j.zo = j.z;
            let mut peor = f32::INFINITY;
            // 300 ticks son ~30 bloques andando: sigue dentro de la plataforma.
            for i in 0..300 {
                j.tick(&nivel, 1.0, 0.0, false, false);
                if i > 80 {
                    peor = peor.min(j.bb.z0);
                }
            }
            println!("altura={altura} z={:.3} pie={:.3} peor={peor:.3} on={}", j.z, j.bb.z0, j.on_ground);
            assert!(
                peor >= altura as f32 + 1.0,
                "altura={altura}: el jugador se hundio hasta pie={peor:.3}"
            );
        }
    }

    #[test]
    fn el_jugador_cae_y_se_queda_sobre_el_suelo() {
        let nivel = Nivel::new(12345);
        let base = nivel.light[128 + 128 * nivel.ancho];
        let mut j = Jugador::new(256, 256, 64);
        j.set_pos(128.5, 128.5, 74.0);
        j.xo = j.x;
        j.yo = j.y;
        j.zo = j.z;
        for i in 0..400 {
            j.tick(&nivel, 0.0, 0.0, false, false);
            if i % 20 == 0 {
                println!("i={i} z={:.3} z0={:.3} on={}", j.z, j.z - 1.62, j.on_ground);
            }
        }
        println!("base={base} z={:.3}", j.z);
        assert!(j.z >= base as f32 + 1.0, "se hundio: z={} base={}", j.z, base);
    }

    #[test]
    fn el_jugador_cae_en_el_mundo_largo_de_013a() {
        let nivel = Nivel::new_013a_dev(12345);
        assert_eq!((nivel.ancho, nivel.alto, nivel.fondo), (32, 512, 64));
        let mut j = Jugador::new(32, 512, 64);
        j.set_pos(16.0, 256.0, 74.0);
        for _ in 0..400 {
            j.tick(&nivel, 0.0, 0.0, false, false);
        }
        // La superficie plana está en z=32; el jugador debe quedar encima, no
        // atravesarla ni caer fuera del mundo.
        assert!(j.z >= 33.0 && j.z < 35.0, "z inesperado en mundo largo: {}", j.z);
    }

    #[test]
    fn las_teclas_de_bloque_son_las_teclas_fisicas_del_original() {
        assert_eq!(bloque_para_tecla("rd-20090515", "1"), Some(tiles::PIEDRA));
        assert_eq!(bloque_para_tecla("rd-20090515", "2"), Some(tiles::TIERRA));
        assert_eq!(bloque_para_tecla("rd-20090515", "3"), Some(tiles::LADRILLO));
        assert_eq!(bloque_para_tecla("rd-20090515", "4"), Some(tiles::MADERA));
        assert_eq!(bloque_para_tecla("rd-20090515", "6"), Some(tiles::ARBUSTO));
        assert_eq!(bloque_para_tecla("rd-161348", "6"), Some(tiles::ARBUSTO));
        // La 5 y la 7 no seleccionan bloque en RubyDung.
        assert_eq!(bloque_para_tecla("rd-20090515", "5"), None);
        assert_eq!(bloque_para_tecla("rd-20090515", "7"), None);
        // rd-160052 no define Bush: la 6 no debe seleccionar nada.
        assert_eq!(bloque_para_tecla("rd-160052", "6"), None);
        // Hasta 0.0.13a_03 no hay teclas para agua, lava ni bedrock: el jar
        // solo asigna 1/2/3/4/6 a piedra, tierra, ladrillo, madera y arbusto.
        for version in [
            "c0.0.12a",
            "c0.0.12a_01",
            "c0.0.12a_02",
            "c0.0.12a_03",
            "c0.0.13a",
            "c0.0.13a_03",
        ] {
            assert_eq!(bloque_para_tecla(version, "7"), None);
            assert_eq!(bloque_para_tecla(version, "8"), None);
            assert_eq!(bloque_para_tecla(version, "9"), None);
        }
        // 0.0.14a estrena las nueve teclas y la paleta con los bloques nuevos.
        assert_eq!(bloque_para_tecla("c0.0.14a", "1"), Some(tiles::PIEDRA));
        assert_eq!(bloque_para_tecla("c0.0.14a", "3"), Some(tiles::TIERRA));
        assert_eq!(bloque_para_tecla("c0.0.14a", "6"), Some(tiles::ARENA));
        assert_eq!(bloque_para_tecla("c0.0.14a", "7"), Some(tiles::GRAVA));
        assert_eq!(bloque_para_tecla("c0.0.14a", "8"), Some(tiles::TRONCO));
        assert_eq!(bloque_para_tecla("c0.0.14a", "9"), Some(tiles::HOJAS));
    }
}

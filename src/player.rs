// Jugador calcado en el Player original de rd-132211.
// Ejes: x, y horizontales; z vertical (arriba). Angulos en grados.

use crate::world::{BlockBox, Level};

pub struct Player {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    // Posicion del tick anterior para interpolar la camara, como moveCameraToPlayer(a)
    // del original. Sin esto la camara avanza a tirones de 60 Hz.
    pub xo: f32,
    pub yo: f32,
    pub zo: f32,
    pub xd: f32,
    pub yd: f32,
    pub zd: f32,
    pub y_rot: f32,
    pub x_rot: f32,
    pub on_ground: bool,
}

impl Player {
    pub fn new(level: &Level) -> Self {
        let mut p = Player {
            x: 0.0, y: 0.0, z: 0.0,
            xo: 0.0, yo: 0.0, zo: 0.0,
            xd: 0.0, yd: 0.0, zd: 0.0,
            y_rot: 0.0, x_rot: 0.0,
            on_ground: false,
        };
        p.reset_pos(level);
        // RD_POS="x,fondo,alto" y RD_ROT="yaw,pitch" colocan la camara donde digan
        // (arnes de careo contra el original)
        if let Ok(v) = std::env::var("RD_POS") {
            let n: Vec<f32> = v.split(',').filter_map(|t| t.trim().parse().ok()).collect();
            if n.len() == 3 {
                p.x = n[0];
                p.y = n[1];
                // RD_POS da la altura del OJO, como el careo del Java; la caja
                // esta centrada 0.72 por debajo del ojo.
                p.z = n[2] - 0.72;
            }
        }
        if let Ok(v) = std::env::var("RD_ROT") {
            let n: Vec<f32> = v.split(',').filter_map(|t| t.trim().parse().ok()).collect();
            if n.len() == 2 {
                p.y_rot = n[0];
                p.x_rot = n[1];
            }
        }
        p.xo = p.x;
        p.yo = p.y;
        p.zo = p.z;
        p
    }

    // Arnes de careo: RD_FIJA=1 hace que RD_POS/RD_ROT manden en cada frame. El
    // raton capturado mueve la camara al traer la ventana al frente y las capturas
    // dejaban de coincidir con las del original.
    pub fn careo_fijar(&mut self) {
        if std::env::var_os("RD_FIJA").is_none() {
            return;
        }
        if let Ok(v) = std::env::var("RD_POS") {
            let n: Vec<f32> = v.split(',').filter_map(|t| t.trim().parse().ok()).collect();
            if n.len() == 3 {
                self.x = n[0];
                self.y = n[1];
                self.z = n[2] - 0.72;
                self.xo = self.x;
                self.yo = self.y;
                self.zo = self.z;
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

    // Igual que resetPos del original: x/z aleatorios en el plano, y(de ellos)=depth+10
    // -> en nuestro eje vertical: z = DEPTH + 10 (siempre sobre el terreno).
    pub fn reset_pos(&mut self, _level: &Level) {
        // resetPos del original: DOS Math.random() independientes, uno para x y
        // otro para el fondo. Con una sola fraccion el jugador salia siempre en
        // la diagonal x == y.
        let fx = spawn_fraction();
        let fy = spawn_fraction();
        self.x = crate::world::WIDTH as f32 * fx;
        self.y = crate::world::HEIGHT as f32 * fy;
        self.z = crate::world::DEPTH as f32 + 10.0;
    }

    // El original pone el ojo en bb.y0 + 1.62; nuestra caja (1.8 de alto) tiene
    // el centro en bb.y0 + 0.9, asi que el ojo queda 0.72 sobre el centro.
    // moveCameraToPlayer(a): interpola xo/yo/zo con la fraccion de tick que
    // sobra. Devuelve (x, alto, fondo), el orden que espera el renderer.
    pub fn eye_pos_at(&self, a: f32) -> (f32, f32, f32) {
        let x = self.xo + (self.x - self.xo) * a;
        let y = self.yo + (self.y - self.yo) * a;
        let z = self.zo + (self.z - self.zo) * a;
        (x, z + 0.72, y)
    }

    // turn(x, y) del original: yRot += x*0.15; xRot -= y*0.15; clamp +-90
    pub fn turn(&mut self, dx: f32, dy: f32) {
        self.y_rot += dx * 0.15;
        self.x_rot -= dy * 0.15;
        if self.x_rot < -90.0 { self.x_rot = -90.0; }
        if self.x_rot > 90.0 { self.x_rot = 90.0; }
    }

    // moveRelative del original, mapeado a nuestro plano (x, y) con z arriba.
    // forward = (sin(yRot), cos(yRot)), right = (cos(yRot), -sin(yRot))
    pub fn move_relative(&mut self, xa: f32, za: f32, accel: f32) {
        let dist = (xa * xa + za * za).sqrt();
        if dist < 0.01 {
            return;
        }
        let d = accel / dist;
        let ya = self.y_rot.to_radians();
        // Coincide con look_at: forward = (sin, -cos), derecha = (cos, sin)
        self.xd += (xa * ya.sin() + za * ya.cos()) * d;
        self.yd += (-xa * ya.cos() + za * ya.sin()) * d;
    }

    pub fn tick(&mut self, level: &Level, f: f32, s: f32, jump: bool, reset: bool) {
        self.xo = self.x;
        self.yo = self.y;
        self.zo = self.z;
        if reset {
            self.reset_pos(level);
        }
        let accel = if self.on_ground { 0.02 } else { 0.005 };
        if jump && self.on_ground {
            self.zd = 0.12;
        }
        self.move_relative(f, s, accel);
        self.zd -= 0.005; // gravedad (eje vertical z)
        self.mover(self.xd, self.yd, self.zd, level);
        self.xd *= 0.91;
        self.zd *= 0.98;
        self.yd *= 0.91;
        if self.on_ground {
            self.xd *= 0.8;
            self.yd *= 0.8;
        }
    }

    pub fn bbox(&self) -> BlockBox {
        BlockBox {
            x0: self.x - 0.3,
            x1: self.x + 0.3,
            y0: self.y - 0.3,
            y1: self.y + 0.3,
            z0: self.z - 0.9,
            z1: self.z + 0.9,
        }
    }

    pub fn mover(&mut self, xa: f32, ya: f32, za: f32, level: &Level) {
        // Player.move del original: recorte de la caja contra los bloques en el orden
        // vertical -> x -> y. Sin atajos: estando dentro de un bloque el clip no te frena
        // (puedes salir subiendo), pero si te frena el bloque solido que tengas encima.
        let (xa0, ya0, za0) = (xa, ya, za);
        let area = self.bbox().expanded(xa, ya, za);
        let cubes = level.cubes(&area);
        let mut bb = self.bbox();

        let mut dz = za;
        for c in &cubes {
            dz = c.clip_z(&bb, dz);
        }
        bb.move_by(0.0, 0.0, dz);

        let mut dx = xa;
        for c in &cubes {
            dx = c.clip_x(&bb, dx);
        }
        bb.move_by(dx, 0.0, 0.0);

        let mut dy = ya;
        for c in &cubes {
            dy = c.clip_y(&bb, dy);
        }
        bb.move_by(0.0, dy, 0.0);

        self.on_ground = za0 != dz && za0 < 0.0;
        if xa0 != dx {
            self.xd = 0.0;
        }
        if za0 != dz {
            self.zd = 0.0;
        }
        if ya0 != dy {
            self.yd = 0.0;
        }

        self.x = (bb.x0 + bb.x1) / 2.0;
        self.y = (bb.y0 + bb.y1) / 2.0;
        self.z = bb.z0 + 0.9;
    }
}

// Punto de aparicion. RD_SPAWN=f fija la fraccion del mapa (el banco la comparte con
// el original, que recibe el mismo valor); sin ella se elige al azar.
fn spawn_fraction() -> f32 {
    if let Ok(v) = std::env::var("RD_SPAWN") {
        if let Ok(f) = v.trim().parse::<f32>() {
            return f.clamp(0.0, 1.0);
        }
    }
    rand()
}

fn rand() -> f32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    ((n % 1000) as f32) / 1000.0
}

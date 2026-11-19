// Entidades de rd-132328: Entity, Player y Zombie, mas el sistema de modelos
// (Vec3/Vertex/Polygon/Cube) que el original descompilado usa para dibujar al
// zombi con la piel char.png.
//
// Ejes como el resto del port: x e y horizontales, z arriba, angulos en grados.
// En el Java original el eje vertical es "y" y la profundidad "z"; aqui se han
// intercambiado (vertical -> z, profundidad -> y) igual que en el resto del motor.

use crate::world::{BlockBox, Mundo};

// ---------------------------------------------------------------- modelos

#[derive(Clone, Copy)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Vec3 { x, y, z }
    }
}

#[derive(Clone, Copy)]
pub struct Vertex {
    pub pos: Vec3,
    pub u: f32,
    pub v: f32,
}

impl Vertex {
    fn new(x: f32, y: f32, z: f32, u: f32, v: f32) -> Self {
        Vertex { pos: Vec3::new(x, y, z), u, v }
    }

    fn remap(&self, u: f32, v: f32) -> Self {
        Vertex { pos: self.pos, u, v }
    }
}

// Polygon con las esquinas ya remapeadas a la piel (64x32 pixeles).
#[derive(Clone, Copy)]
pub struct Polygon {
    pub vertices: [Vertex; 4],
}

impl Polygon {
    fn new(vertices: [Vertex; 4], u0: f32, v0: f32, u1: f32, v1: f32) -> Self {
        // El original asigna: [0]=(u1,v0) [1]=(u0,v0) [2]=(u0,v1) [3]=(u1,v1)
        Polygon {
            vertices: [
                vertices[0].remap(u1, v0),
                vertices[1].remap(u0, v0),
                vertices[2].remap(u0, v1),
                vertices[3].remap(u1, v1),
            ],
        }
    }
}

// Caja del modelo: 6 caras con su trozo de piel, y su transformacion.
pub struct Cube {
    pub polygons: [Polygon; 6],
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub x_rot: f32,
    pub y_rot: f32,
    pub z_rot: f32,
}

impl Cube {
    pub fn new(_x_tex: f32, _y_tex: f32) -> Self {
        // add_box se llama despues; aqui solo hace falta el sitio.
        let v = Vertex::new(0.0, 0.0, 0.0, 0.0, 0.0);
        let p = Polygon::new([v; 4], 0.0, 0.0, 0.0, 0.0);
        Cube {
            polygons: [p; 6],
            x: 0.0,
            y: 0.0,
            z: 0.0,
            x_rot: 0.0,
            y_rot: 0.0,
            z_rot: 0.0,
        }
    }

    // Cube.addBox del original. Coordenadas de modelo: x derecha, y arriba, z
    // hacia atras; luego se rotan y se escala todo junto.
    pub fn add_box(&mut self, x0: f32, y0: f32, z0: f32, w: i32, h: i32, d: i32, x_tex: f32, y_tex: f32) {
        let (wf, hf, df) = (w as f32, h as f32, d as f32);
        let x1 = x0 + wf;
        let y1 = y0 + hf;
        let z1 = z0 + df;
        let u0 = Vertex::new(x0, y0, z0, 0.0, 0.0);
        let u1 = Vertex::new(x1, y0, z0, 0.0, 8.0);
        let u2 = Vertex::new(x1, y1, z0, 8.0, 8.0);
        let u3 = Vertex::new(x0, y1, z0, 8.0, 0.0);
        let l0 = Vertex::new(x0, y0, z1, 0.0, 0.0);
        let l1 = Vertex::new(x1, y0, z1, 0.0, 8.0);
        let l2 = Vertex::new(x1, y1, z1, 8.0, 8.0);
        let l3 = Vertex::new(x0, y1, z1, 8.0, 0.0);
        self.polygons[0] = Polygon::new([l1, u1, u2, l2], x_tex + df + wf, y_tex + df, x_tex + df + wf + df, y_tex + df + hf);
        self.polygons[1] = Polygon::new([u0, l0, l3, u3], x_tex, y_tex + df, x_tex + df, y_tex + df + hf);
        self.polygons[2] = Polygon::new([l1, l0, u0, u1], x_tex + df, y_tex, x_tex + df + wf, y_tex + df);
        self.polygons[3] = Polygon::new([u2, u3, l3, l2], x_tex + df + wf, y_tex, x_tex + df + wf + wf, y_tex + df);
        self.polygons[4] = Polygon::new([u1, u0, u3, u2], x_tex + df, y_tex + df, x_tex + df + wf, y_tex + df + hf);
        self.polygons[5] = Polygon::new([l0, l1, l2, l3], x_tex + df + wf + df, y_tex + df, x_tex + df + wf + df + wf, y_tex + df + hf);
    }

    pub fn set_pos(&mut self, x: f32, y: f32, z: f32) {
        self.x = x;
        self.y = y;
        self.z = z;
    }
}

// ---------------------------------------------------------------- entidades

pub struct Entity {
    pub xo: f32,
    pub yo: f32,
    pub zo: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub xd: f32,
    pub yd: f32,
    pub zd: f32,
    pub y_rot: f32,
    pub x_rot: f32,
    pub bb: BlockBox,
    pub on_ground: bool,
    pub height_offset: f32,
}

impl Entity {
    pub fn new(level: &dyn Mundo) -> Self {
        let mut e = Entity {
            xo: 0.0, yo: 0.0, zo: 0.0,
            x: 0.0, y: 0.0, z: 0.0,
            xd: 0.0, yd: 0.0, zd: 0.0,
            y_rot: 0.0, x_rot: 0.0,
            bb: BlockBox { x0: 0.0, x1: 0.0, y0: 0.0, y1: 0.0, z0: 0.0, z1: 0.0 },
            on_ground: false,
            height_offset: 0.0,
        };
        e.reset_pos(level);
        e
    }

    // resetPos del original: x aleatoria en todo el ancho, y (vertical) a
    // depth + 10, z aleatoria en todo el fondo. Antes la profundidad se sorteaba
    // solo hasta 64 en vez de 256, asi que jugador y zombis aparecian siempre en
    // una franja estrecha del mapa.
    pub fn reset_pos(&mut self, _level: &dyn Mundo) {
        let x = randf() * crate::world::WIDTH as f32;
        let y = randf() * crate::world::HEIGHT as f32;
        self.set_pos(x, y, crate::world::DEPTH as f32 + 10.0);
    }

    // setPos: caja de 0.6 x 0.6 de planta y 1.8 de alto centrada en (x,y,z).
    pub fn set_pos(&mut self, x: f32, y: f32, z: f32) {
        self.x = x;
        self.y = y;
        self.z = z;
        let (w, h) = (0.3f32, 0.9f32);
        self.bb = BlockBox {
            x0: x - w, x1: x + w,
            y0: y - w, y1: y + w,
            z0: z - h, z1: z + h,
        };
    }

    pub fn turn(&mut self, dx: f32, dy: f32) {
        self.y_rot += dx * 0.15;
        self.x_rot -= dy * 0.15;
        if self.x_rot < -90.0 { self.x_rot = -90.0; }
        if self.x_rot > 90.0 { self.x_rot = 90.0; }
    }

    pub fn tick_prev(&mut self) {
        self.xo = self.x;
        self.yo = self.y;
        self.zo = self.z;
    }

    // Entity.move: vertical -> x -> profundidad, y luego resitua el centro.
    pub fn mover(&mut self, xa: f32, ya: f32, za: f32, level: &dyn Mundo) {
        let (xa0, ya0, za0) = (xa, ya, za);
        let area = self.bb.expanded(xa, ya, za);
        let cubes = level.cajas(&area);
        let mut bb = self.bb;

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
        if xa0 != dx { self.xd = 0.0; }
        if za0 != dz { self.zd = 0.0; }
        if ya0 != dy { self.yd = 0.0; }

        self.bb = bb;
        self.x = (bb.x0 + bb.x1) / 2.0;
        self.z = bb.z0 + self.height_offset;
        self.y = (bb.y0 + bb.y1) / 2.0;
    }

    // moveRelative: igual que en el jugador ya portado (misma convencion de yRot).
    pub fn move_relative(&mut self, xa: f32, za: f32, speed: f32) {
        let dist = (xa * xa + za * za).sqrt();
        if dist < 0.01 {
            return;
        }
        let d = speed / dist;
        let yr = self.y_rot.to_radians();
        self.xd += (xa * yr.sin() + za * yr.cos()) * d;
        self.yd += (-xa * yr.cos() + za * yr.sin()) * d;
    }
}

pub struct PlayerE {
    pub e: Entity,
}

impl PlayerE {
    pub fn new(level: &dyn Mundo) -> Self {
        let mut e = Entity::new(level);
        e.height_offset = 1.62;
        PlayerE { e }
    }

    // Player.tick: input y fisica. f = adelante/atras, s = lateral, jump.
    pub fn tick(&mut self, level: &dyn Mundo, f: f32, s: f32, jump: bool) {
        self.e.tick_prev();
        if jump && self.e.on_ground {
            self.e.zd = 0.12;
        }
        let accel = if self.e.on_ground { 0.02 } else { 0.005 };
        self.e.move_relative(f, s, accel);
        self.e.zd -= 0.005;
        let (xd, yd, zd) = (self.e.xd, self.e.yd, self.e.zd);
        self.e.mover(xd, yd, zd, level);
        self.e.xd *= 0.91;
        self.e.zd *= 0.98;
        self.e.yd *= 0.91;
        if self.e.on_ground {
            self.e.xd *= 0.8;
            self.e.yd *= 0.8;
        }
    }
}

// ------------------------------------------------------------------ zombi

pub struct Zombie {
    pub e: Entity,
    pub head: Cube,
    pub body: Cube,
    pub arm0: Cube,
    pub arm1: Cube,
    pub leg0: Cube,
    pub leg1: Cube,
    pub rot: f32,
    pub time_offs: f32,
    pub speed: f32,
    pub rot_a: f32,
}

impl Zombie {
    pub fn new(level: &dyn Mundo, x: f32, y: f32, z: f32) -> Self {
        let mut e = Entity::new(level);
        // rd-132328 asigna x/y/z directamente despues de resetPos y deja la caja
        // como estaba; su original hace lo mismo. El legacy, en cambio, llama a
        // setPos (que tambien mueve la caja); su G lo fija en legacy/juego.rs.
        e.x = x;
        e.y = y;
        e.z = z;
        e.height_offset = 0.0;

        let mut head = Cube::new(0.0, 0.0);
        head.add_box(-4.0, -8.0, -4.0, 8, 8, 8, 0.0, 0.0);
        let mut body = Cube::new(16.0, 16.0);
        body.add_box(-4.0, 0.0, -2.0, 8, 12, 4, 16.0, 16.0);
        let mut arm0 = Cube::new(40.0, 16.0);
        arm0.add_box(-3.0, -2.0, -2.0, 4, 12, 4, 40.0, 16.0);
        arm0.set_pos(-5.0, 2.0, 0.0);
        let mut arm1 = Cube::new(40.0, 16.0);
        arm1.add_box(-1.0, -2.0, -2.0, 4, 12, 4, 40.0, 16.0);
        arm1.set_pos(5.0, 2.0, 0.0);
        let mut leg0 = Cube::new(0.0, 16.0);
        leg0.add_box(-2.0, 0.0, -2.0, 4, 12, 4, 0.0, 16.0);
        leg0.set_pos(-2.0, 12.0, 0.0);
        let mut leg1 = Cube::new(0.0, 16.0);
        leg1.add_box(-2.0, 0.0, -2.0, 4, 12, 4, 0.0, 16.0);
        leg1.set_pos(2.0, 12.0, 0.0);

        Zombie {
            e,
            head,
            body,
            arm0,
            arm1,
            leg0,
            leg1,
            rot: (randf() * std::f32::consts::PI * 2.0),
            time_offs: randf() * 1239813.0,
            speed: 1.0,
            rot_a: (randf() + 1.0) * 0.01,
        }
    }

    pub fn tick(&mut self, level: &dyn Mundo) {
        self.e.tick_prev();
        self.rot += self.rot_a;
        self.rot_a *= 0.99;
        self.rot_a += (randf() - randf()) * randf() * randf() * 0.01;
        let xa = self.rot.sin();
        let za = self.rot.cos();
        if self.e.on_ground && randf() < 0.01 {
            self.e.zd = 0.12;
        }
        let accel = if self.e.on_ground { 0.02 } else { 0.005 };
        // moveRelative del original recibe (strafe, avance): el zombi le pasa
        // (sin(rot), cos(rot)). Nuestro move_relative recibe (avance, strafe) y
        // su avance con yaw 0 apunta a -profundidad, asi que hay que invertir el
        // avance. Antes se pasaban (sin, cos) y el zombi caminaba de lado,
        // perpendicular a su cuerpo.
        self.e.move_relative(-za, xa, accel);
        self.e.zd -= 0.005;
        let (xd, yd, zd) = (self.e.xd, self.e.yd, self.e.zd);
        self.e.mover(xd, yd, zd, level);
        self.e.xd *= 0.91;
        self.e.zd *= 0.98;
        self.e.yd *= 0.91;
        if self.e.z > 100.0 {
            self.e.reset_pos(level);
        }
        if self.e.on_ground {
            self.e.xd *= 0.8;
            self.e.yd *= 0.8;
        }
    }

    // Vertices (pos + uv) del modelo en coordenadas de mundo, con la animacion
    // del original y la interpolacion `a` entre el tick anterior y el actual.
    pub fn vertices(&self, a: f32, out: &mut Vec<[f32; 5]>) {
        let time = (now_nanos() as f64 / 1.0e9) * 10.0 * self.speed as f64 + self.time_offs as f64;
        self.vertices_at(time, a, out);
    }

    /// Igual que `vertices`, pero con el tiempo de animacion fijado. Se usa en
    /// el arnes de careo para comparar la misma pose contra el original.
    pub fn vertices_at(&self, time: f64, a: f32, out: &mut Vec<[f32; 5]>) {
        let size = 0.058333334f32;
        let yy = (-((time * 0.6662).sin().abs()) as f32) * 5.0 - 23.0;
        let px = self.e.xo + (self.e.x - self.e.xo) * a;
        let py = self.e.yo + (self.e.y - self.e.yo) * a;
        let pz = self.e.zo + (self.e.z - self.e.zo) * a;

        // El original guarda estos valores en RADIANES: Cube.render los multiplica
        // por 57.29578 al llamar a glRotatef. Nuestros helpers rot_x/rot_y/rot_z
        // reciben grados, asi que hay que convertirlos; si se pasan tal cual, el
        // zombi mueve los brazos 57 veces menos y se queda tieso.
        let head_yr = (((time * 0.83).sin()) as f32).to_degrees();
        let head_xr = (time.sin() as f32 * 0.8).to_degrees();
        let arm0_xr = ((((time * 0.6662 + std::f64::consts::PI).sin()) as f32) * 2.0).to_degrees();
        let arm0_zr = ((((time * 0.2312).sin()) as f32 + 1.0) * 1.0).to_degrees();
        let arm1_xr = ((((time * 0.6662).sin()) as f32) * 2.0).to_degrees();
        let arm1_zr = ((((time * 0.2812).sin()) as f32 - 1.0) * 1.0).to_degrees();
        let leg0_xr = ((((time * 0.6662).sin()) as f32) * 1.4).to_degrees();
        let leg1_xr = ((((time * 0.6662 + std::f64::consts::PI).sin()) as f32) * 1.4).to_degrees();

        // Transformacion comun: traslacion + escala + giro del zombi.
        // En el original: scale(1,-1,1), scale(size), translate(0,yy,0),
        // rotateY(rot*57.3+180). Nuestro eje vertical es z, asi que el -1 va en z.
        let rot_y = (self.rot.to_degrees() + 180.0).to_radians();
        let (sr, cr) = rot_y.sin_cos();

        let piezas: [(&Cube, (f32, f32, f32)); 6] = [
            (&self.head, (head_xr, head_yr, 0.0)),
            (&self.body, (0.0, 0.0, 0.0)),
            (&self.arm0, (arm0_xr, 0.0, arm0_zr)),
            (&self.arm1, (arm1_xr, 0.0, arm1_zr)),
            (&self.leg0, (leg0_xr, 0.0, 0.0)),
            (&self.leg1, (leg1_xr, 0.0, 0.0)),
        ];

        for (cube, (xr, yr, zr)) in piezas.iter() {
            for poly in cube.polygons.iter() {
                // Triangulo (0,1,2) y (0,2,3) con los mismos vertices.
                for idx in [0usize, 1, 2, 0, 2, 3] {
                    let v = poly.vertices[idx];
                    // 1) rotacion de la pieza (grados) y traslacion dentro del modelo
                    let (ax, ay, az) = rot_x(v.pos, *xr);
                    let (bx, by, bz) = rot_y_(ax, ay, az, *yr);
                    let (cx, cy, cz) = rot_z(bx, by, bz, *zr);
                    let mx = cx + cube.x;
                    let my = cy + cube.y;
                    let mz = cz + cube.z;
                    // 2) a mundo: escala, el -1 vertical del original metido en
                    //    translate(0, yy, 0), y giro del zombi alrededor del eje
                    //    vertical. Salida en coords de PANTALLA (x, arriba, fondo).
                    let sx = mx * size;
                    let sd = mz * size;
                    let su = -(my + yy) * size;
                    let rx = sx * cr + sd * sr;
                    let rd = -sx * sr + sd * cr;
                    out.push([px + rx, pz + su, py + rd, v.u / 64.0, v.v / 32.0]);
                }
            }
        }
    }
}

fn rot_x(p: Vec3, deg: f32) -> (f32, f32, f32) {
    let (s, c) = deg.to_radians().sin_cos();
    (p.x, p.y * c - p.z * s, p.y * s + p.z * c)
}

fn rot_y_(x: f32, y: f32, z: f32, deg: f32) -> (f32, f32, f32) {
    let (s, c) = deg.to_radians().sin_cos();
    (x * c + z * s, y, -x * s + z * c)
}

fn rot_z(x: f32, y: f32, z: f32, deg: f32) -> (f32, f32, f32) {
    let (s, c) = deg.to_radians().sin_cos();
    (x * c - y * s, x * s + y * c, z)
}

fn now_nanos() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0)
}

// Math.random() del original: basta con algo uniforme en [0,1).
pub fn randf() -> f32 {
    use std::sync::atomic::{AtomicU32, Ordering};
    static SEED: AtomicU32 = AtomicU32::new(0x9E3779B9);
    let mut s = SEED.load(Ordering::Relaxed);
    s ^= s << 13;
    s ^= s >> 17;
    s ^= s << 5;
    SEED.store(s, Ordering::Relaxed);
    (s as f32) / (u32::MAX as f32)
}

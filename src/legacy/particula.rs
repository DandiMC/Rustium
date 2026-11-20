// Particle / ParticleEngine de la familia B (rd-160052 .. rd-161348).
//
// Al romper un bloque el original suelta 4x4x4 = 64 particulas con la textura
// del tile, con fisica propia (gravedad 0.04, roce 0.98, 0.7 al tocar el suelo),
// y las dibuja como quads siempre de cara a la camara.

use super::level::Nivel;
use crate::render::Vertex;
use crate::world::BlockBox;

pub struct Particula {
    pub xo: f32,
    pub yo: f32,
    pub zo: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    xd: f32,
    yd: f32,
    zd: f32,
    bb: BlockBox,
    on_ground: bool,
    pub tex: u32,
    uo: f32,
    vo: f32,
    age: i32,
    lifetime: i32,
    size: f32,
    pub borrada: bool,
}

impl Particula {
    pub fn new(x: f32, y: f32, z: f32, xa: f32, ya: f32, za: f32, tex: u32) -> Self {
        let mut p = Particula {
            xo: 0.0, yo: 0.0, zo: 0.0,
            x: 0.0, y: 0.0, z: 0.0,
            xd: 0.0, yd: 0.0, zd: 0.0,
            bb: BlockBox { x0: 0.0, x1: 0.0, y0: 0.0, y1: 0.0, z0: 0.0, z1: 0.0 },
            on_ground: false,
            tex,
            uo: 0.0, vo: 0.0,
            age: 0,
            lifetime: 0,
            size: 1.0,
            borrada: false,
        };
        // setSize(0.2, 0.2) + heightOffset = bbHeight/2 + setPos
        p.set_pos(x, y, z);
        let (r1, r2, r3) = (randf(), randf(), randf());
        p.xd = xa + (r1 * 2.0 - 1.0) * 0.4;
        p.yd = ya + (r2 * 2.0 - 1.0) * 0.4;
        p.zd = za + (r3 * 2.0 - 1.0) * 0.4;
        let speed = (randf() + randf() + 1.0) * 0.15;
        let dd = (p.xd * p.xd + p.yd * p.yd + p.zd * p.zd).sqrt();
        p.xd = p.xd / dd * speed * 0.4;
        p.yd = p.yd / dd * speed * 0.4 + 0.1;
        p.zd = p.zd / dd * speed * 0.4;
        p.uo = randf() * 3.0;
        p.vo = randf() * 3.0;
        p.size = randf() * 0.5 + 0.5;
        p.lifetime = (4.0 / (randf() * 0.9 + 0.1)) as i32;
        p
    }

    fn set_pos(&mut self, x: f32, y: f32, z: f32) {
        self.x = x;
        self.y = y;
        self.z = z;
        let (w, h) = (0.1f32, 0.1f32);
        self.bb = BlockBox {
            x0: x - w, x1: x + w,
            y0: y - w, y1: y + w,
            z0: z - h, z1: z + h,
        };
    }

    pub fn tick(&mut self, nivel: &Nivel) {
        self.xo = self.x;
        self.yo = self.y;
        self.zo = self.z;
        // `this.age++ >= lifetime` del original: compara antes de incrementar,
        // asi que la particula vive exactamente `lifetime` ticks.
        if self.age >= self.lifetime {
            self.borrada = true;
        }
        self.age += 1;
        self.zd -= 0.04;
        let (xa, ya, za) = (self.xd, self.yd, self.zd);
        // Entity.move con la caja de 0.2x0.2
        let area = self.bb.expanded(xa, ya, za);
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

        self.on_ground = za != dz && za < 0.0;
        if xa != dx { self.xd = 0.0; }
        if za != dz { self.zd = 0.0; }
        if ya != dy { self.yd = 0.0; }

        self.bb = bb;
        self.x = (bb.x0 + bb.x1) / 2.0;
        self.y = (bb.y0 + bb.y1) / 2.0;
        self.z = bb.z0 + 0.1;
        self.xd *= 0.98;
        self.yd *= 0.98;
        self.zd *= 0.98;
        if self.on_ground {
            self.xd *= 0.7;
            self.yd *= 0.7;
        }
    }

    // Quad de cara a la camara. `der` y `arr` son los ejes derecha/arriba de la
    // camara; se anade a la malla dinamica con la textura del tile (y el trozo
    // aleatorio de 1/4 de tile que usa el original).
    pub fn vertices(&self, a: f32, der: [f32; 3], arr: [f32; 3], verts: &mut Vec<Vertex>, idx: &mut Vec<u32>) {
        let u0 = ((self.tex % 16) as f32 + self.uo / 4.0) / 16.0;
        let u1 = u0 + 0.015609375;
        let v0 = ((self.tex / 16) as f32 + self.vo / 4.0) / 16.0;
        let v1 = v0 + 0.015609375;
        let r = 0.1 * self.size;
        let cx = self.xo + (self.x - self.xo) * a;
        let cy = self.yo + (self.y - self.yo) * a;
        let cz = self.zo + (self.z - self.zo) * a;
        let base = verts.len() as u32;
        // [-der-arr], [-der+arr], [+der+arr], [+der-arr]
        let signs = [(-1.0f32, -1.0f32), (-1.0, 1.0), (1.0, 1.0), (1.0, -1.0)];
        // Orden de vertices de Particle.render: (-r,-r) es el vertice de u1/v0,
        // no el de u0/v1; con el orden anterior la textura salia girada 180.
        let uvs = [[u1, v0], [u1, v1], [u0, v1], [u0, v0]];
        for (i, (sd, sa)) in signs.iter().enumerate() {
            verts.push(Vertex {
                pos: [
                    cx + der[0] * r * sd + arr[0] * r * sa,
                    cz + der[1] * r * sd + arr[1] * r * sa,
                    cy + der[2] * r * sd + arr[2] * r * sa,
                ],
                uv: uvs[i],
                shade: 0.8,
            });
        }
        // De doble cara: el original nunca activa GL_CULL_FACE, asi que el quad
        // se ve desde los dos lados igual que en el juego de verdad.
        idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        idx.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
    }
}

// Random uniforme en [0,1) para las particulas (Math.random del original).
fn randf() -> f32 {
    crate::entity::randf()
}

// Tabla de 4x4x4 = 64 particulas, como Tile.destroy.
pub fn destruir(nivel_id_tex: u32, x: i32, y: i32, z: i32, out: &mut Vec<Particula>) {
    const SD: i32 = 4;
    for xx in 0..SD {
        for yy in 0..SD {
            for zz in 0..SD {
                let xp = x as f32 + (xx as f32 + 0.5) / SD as f32;
                let yp = y as f32 + (yy as f32 + 0.5) / SD as f32;
                let zp = z as f32 + (zz as f32 + 0.5) / SD as f32;
                out.push(Particula::new(
                    xp, yp, zp,
                    xp - x as f32 - 0.5,
                    yp - y as f32 - 0.5,
                    zp - z as f32 - 0.5,
                    nivel_id_tex,
                ));
            }
        }
    }
}

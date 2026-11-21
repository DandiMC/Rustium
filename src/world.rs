// Mundo voxel 256x256x64, calcado en la estructura de rd-132211 (RubyDung).

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};

pub const WIDTH: usize = 256;
pub const HEIGHT: usize = 256;
pub const DEPTH: usize = 64;

pub const AIR: u8 = 0;
// El original solo tiene un bloque solido (id 1) y decide la textura por altura en
// Chunk.rebuild (capa superior = tile 0, resto = tile 1). Aqui igual: el id no
// distingue pasto de roca, lo hace el mallado.
pub const GRASS: u8 = 1;

// Brillo de una cara que tiene bloque solido por encima en su columna (0.8 en el
// bytecode de Level.getBrightness). Antes se usaba 0.52, que era el resultado de
// medir una captura a ~2 bloques; ese 0.52 es 0.8 * exp(-0.2 * 2.18), o sea el
// brillo por la niebla del segundo pase. Ahora la niebla se aplica de verdad
// (GL_EXP, densidad 0.2) y aqui queda el 0.8 limpio.
pub const SHADED: f32 = 0.8;

pub struct Level {
    pub blocks: Vec<u8>,
    // lightDepths del original: altura del bloque solido mas alto de cada columna
    // (indice x + y*WIDTH). Base de las sombras: lo que queda por debajo va a SHADED.
    pub light_depths: Vec<i32>,
}

// Caja alineada a los ejes, equivalente al AABB del original.
#[derive(Clone, Copy, Debug)]
pub struct BlockBox {
    pub x0: f32,
    pub x1: f32,
    pub y0: f32,
    pub y1: f32,
    pub z0: f32,
    pub z1: f32,
}

impl BlockBox {
    // AABB.grow: agranda la caja por los dos lados. pick() la usa para limitar
    // el alcance; expand() solo alarga el lado hacia el que se mueve y dejaba
    // fuera los bloques por debajo de los pies.
    pub fn grown(&self, x: f32, y: f32, z: f32) -> BlockBox {
        BlockBox {
            x0: self.x0 - x,
            x1: self.x1 + x,
            y0: self.y0 - y,
            y1: self.y1 + y,
            z0: self.z0 - z,
            z1: self.z1 + z,
        }
    }

    // AABB.expand: agranda la caja solo hacia el lado en el que se mueve.
    pub fn expanded(&self, x: f32, y: f32, z: f32) -> BlockBox {
        let mut b = *self;
        if x < 0.0 { b.x0 += x; } else { b.x1 += x; }
        if y < 0.0 { b.y0 += y; } else { b.y1 += y; }
        if z < 0.0 { b.z0 += z; } else { b.z1 += z; }
        b
    }

    pub fn move_by(&mut self, x: f32, y: f32, z: f32) {
        self.x0 += x;
        self.x1 += x;
        self.y0 += y;
        self.y1 += y;
        self.z0 += z;
        self.z1 += z;
    }

    // AABB.clip*Collide: recorta el desplazamiento para no entrar en este bloque.
    // Si la caja ya esta dentro (atrapado), no la frena: igual que el original.
    pub fn clip_x(&self, bb: &BlockBox, mut d: f32) -> f32 {
        if bb.y1 <= self.y0 || bb.y0 >= self.y1 {
            return d;
        }
        if bb.z1 <= self.z0 || bb.z0 >= self.z1 {
            return d;
        }
        if d > 0.0 && bb.x1 <= self.x0 {
            let v = self.x0 - bb.x1;
            if v < d { d = v; }
        }
        if d < 0.0 && bb.x0 >= self.x1 {
            let v = self.x1 - bb.x0;
            if v > d { d = v; }
        }
        d
    }

    pub fn clip_y(&self, bb: &BlockBox, mut d: f32) -> f32 {
        if bb.x1 <= self.x0 || bb.x0 >= self.x1 {
            return d;
        }
        if bb.z1 <= self.z0 || bb.z0 >= self.z1 {
            return d;
        }
        if d > 0.0 && bb.y1 <= self.y0 {
            let v = self.y0 - bb.y1;
            if v < d { d = v; }
        }
        if d < 0.0 && bb.y0 >= self.y1 {
            let v = self.y1 - bb.y0;
            if v > d { d = v; }
        }
        d
    }

    // Eje vertical en nuestro mundo (el "y" del original).
    pub fn clip_z(&self, bb: &BlockBox, mut d: f32) -> f32 {
        if bb.x1 <= self.x0 || bb.x0 >= self.x1 {
            return d;
        }
        if bb.y1 <= self.y0 || bb.y0 >= self.y1 {
            return d;
        }
        if d > 0.0 && bb.z1 <= self.z0 {
            let v = self.z0 - bb.z1;
            if v < d { d = v; }
        }
        if d < 0.0 && bb.z0 >= self.z1 {
            let v = self.z1 - bb.z0;
            if v > d { d = v; }
        }
        d
    }
}

impl Level {
    pub fn new() -> Self {
        let mut blocks = vec![0u8; WIDTH * HEIGHT * DEPTH];
        Self::generate(&mut blocks);
        let mut level = Level {
            blocks,
            light_depths: vec![0; WIDTH * HEIGHT],
        };
        level.recalc_light();
        // El original carga level.dat justo despues de generar y de calcular la
        // luz; si el fichero no existe o no mide lo mismo, se queda el mundo nuevo.
        level.load();
        // RD_DUMP=1 vuelca bloques y luces como el Java parcheado (careo)
        if std::env::var("RD_DUMP").is_ok() {
            let _ = std::fs::write("bloques_port.raw", &level.blocks);
            let mut luces = Vec::with_capacity(level.light_depths.len() * 4);
            for d in &level.light_depths {
                luces.extend_from_slice(&d.to_be_bytes());
            }
            let _ = std::fs::write("luces_port.raw", &luces);
        }
        level
    }

    /// Crea un nivel con los bloques ya generados, sin cargar level.dat.
    /// Lo usa rd-131655, que no guarda el mundo.
    pub fn from_blocks(blocks: Vec<u8>) -> Self {
        debug_assert_eq!(blocks.len(), WIDTH * HEIGHT * DEPTH);
        let mut level = Level {
            blocks,
            light_depths: vec![0; WIDTH * HEIGHT],
        };
        level.recalc_light();
        level
    }

    fn recalc_light(&mut self) {
        for x in 0..WIDTH as i32 {
            for y in 0..HEIGHT as i32 {
                self.update_light_depth(x, y);
            }
        }
    }

    // calcLightDepths del original, para una sola columna: baja desde el techo
    // mientras no haya bloque solido y guarda la altura del primero que encuentre.
    fn update_light_depth(&mut self, x: i32, y: i32) {
        if x < 0 || y < 0 || x >= WIDTH as i32 || y >= HEIGHT as i32 {
            return;
        }
        let mut z = DEPTH as i32 - 1;
        while z > 0 && !self.is_solid(x, y, z) {
            z -= 1;
        }
        self.light_depths[y as usize * WIDTH + x as usize] = z;
    }

    fn idx(x: usize, y: usize, z: usize) -> usize {
        // Java usa (alto * height + profundidad) * width + x; aqui x/y son
        // horizontales y z la altura, asi que el orden equivalente es este.
        (z * HEIGHT + y) * WIDTH + x
    }

    pub fn get(&self, x: i32, y: i32, z: i32) -> u8 {
        // Fuera del mapa (x/y) es aire: se puede salir del borde y caer, como el original.
        if x < 0 || y < 0 || z < 0 || x >= WIDTH as i32 || y >= HEIGHT as i32 || z >= DEPTH as i32 {
            return AIR;
        }
        self.blocks[Self::idx(x as usize, y as usize, z as usize)]
    }

    pub fn set(&mut self, x: i32, y: i32, z: i32, v: u8) {
        if x < 0 || y < 0 || z < 0 || x >= WIDTH as i32 || y >= HEIGHT as i32 || z >= DEPTH as i32 {
            return;
        }
        self.blocks[Self::idx(x as usize, y as usize, z as usize)] = v;
        // Si se rompe o se coloca un bloque, la columna cambia de altura de luz.
        self.update_light_depth(x, y);
    }

    pub fn is_solid(&self, x: i32, y: i32, z: i32) -> bool {
        self.get(x, y, z) != AIR
    }

    // Level.getBrightness del original: 0.8 si el bloque esta por debajo del bloque
    // solido mas alto de su columna (bajo tierra), 1.0 si esta a la altura de la
    // superficie o por encima. Fuera del mapa devuelve 1.0.
    //
    // El descompilado dice 0.8, pero el original en pantalla no da ese numero: medido
    // sobre capturas, el pasto bajo un voladizo sale en 83 frente a 160 del pasto
    // abierto (0.52) y la cara 0.8 de un bloque tapado 44 frente a 86 (0.52). Se usa
    // ese valor medido, que reproduce las dos zonas a la vez.
    pub fn brightness(&self, x: i32, y: i32, z: i32) -> f32 {
        if x < 0 || y < 0 || z < 0
            || x >= WIDTH as i32
            || y >= HEIGHT as i32
            || z >= DEPTH as i32
        {
            return 1.0;
        }
        let top = self.light_depths[y as usize * WIDTH + x as usize];
        if z < top { SHADED } else { 1.0 }
    }

    // Level.getCubes del original: cajas de los bloques solidos que tocan el area.
    pub fn cubes(&self, area: &BlockBox) -> Vec<BlockBox> {
        let mut out = Vec::new();
        for bx in (area.x0.floor() as i32)..=(area.x1.floor() as i32) {
            for by in (area.y0.floor() as i32)..=(area.y1.floor() as i32) {
                for bz in (area.z0.floor() as i32)..=(area.z1.floor() as i32) {
                    if self.is_solid(bx, by, bz) {
                        out.push(BlockBox {
                            x0: bx as f32,
                            x1: bx as f32 + 1.0,
                            y0: by as f32,
                            y1: by as f32 + 1.0,
                            z0: bz as f32,
                            z1: bz as f32 + 1.0,
                        });
                    }
                }
            }
        }
        out
    }

    // Generacion calcada del original rd-132211: mundo plano, solido hasta
    // z <= depth*2/3. La capa superior (z == depth*2/3) es pasto; el resto roca.
    // Altura de pilar del escenario de estres. Debe dar EXACTAMENTE lo mismo que
    // analisis/mcmod/gen_estres.py, que es quien escribe el level.dat del original.
    fn pillar_height(x: u32, y: u32) -> u32 {
        let mut v = x
            .wrapping_mul(0x9E37_79B1)
            .wrapping_add(y.wrapping_mul(0x85EB_CA77))
            .wrapping_add(0x1234_5678);
        v ^= v >> 15;
        v = v.wrapping_mul(0x2545_F491);
        v ^= v >> 13;
        v & 3
    }

    /// Original: Level.load() lee el array de bloques de un level.dat gzip.
    /// Si el fichero no existe, no es gzip o no tiene el tamano exacto, se
    /// conserva el mundo recien generado y se devuelve false.
    pub fn load(&mut self) -> bool {
        let Ok(file) = File::open("level.dat") else {
            return false;
        };
        let mut data = Vec::with_capacity(self.blocks.len());
        let mut gz = GzDecoder::new(BufReader::new(file));
        if gz.read_to_end(&mut data).is_err() || data.len() != self.blocks.len() {
            return false;
        }
        self.blocks.copy_from_slice(&data);
        self.recalc_light();
        true
    }

    /// Original: Level.save() escribe el array de bloques en level.dat, gzip.
    pub fn save(&self) -> std::io::Result<()> {
        let file = File::create("level.dat")?;
        let mut gz = GzEncoder::new(BufWriter::new(file), Compression::default());
        gz.write_all(&self.blocks)?;
        gz.finish()?.flush()
    }

    fn generate(blocks: &mut [u8]) {
        let top = (DEPTH * 2 / 3) as i32;
        for x in 0..WIDTH {
            for y in 0..HEIGHT {
                for z in 0..DEPTH as i32 {
                    let i = Self::idx(x, y, z as usize);
                    blocks[i] = if z <= top { GRASS } else { AIR };
                }
            }
        }
        // Escenario de estres: RUBYDUNG_ESTRES=1 llena el mapa de pilares, el mismo
        // mundo que escribe analisis/mcmod/gen_estres.py en el level.dat.
        if std::env::var_os("RUBYDUNG_ESTRES").is_some() {
            for x in 0..WIDTH {
                for y in 0..HEIGHT {
                    let h = Self::pillar_height(x as u32, y as u32);
                    for k in 1..=h {
                        let z = top as usize + k as usize;
                        if z < DEPTH {
                            blocks[Self::idx(x, y, z)] = GRASS;
                        }
                    }
                }
            }
        }
    }

    fn hash(x: i32, y: i32, seed: i32) -> f32 {
        let mut h = (x as u32).wrapping_mul(374761393)
            ^ (y as u32).wrapping_mul(668265263)
            ^ (seed as u32).wrapping_mul(2246822519);
        h = h.wrapping_mul(3266489917);
        h ^= h >> 16;
        (h & 0xFFFFFF) as f32 / 0xFFFFFF as f32
    }

    fn smooth(t: f32) -> f32 {
        t * t * (3.0 - 2.0 * t)
    }

    fn value_noise(x: f32, y: f32, seed: i32) -> f32 {
        let xi = x.floor() as i32;
        let yi = y.floor() as i32;
        let xf = x - xi as f32;
        let yf = y - yi as f32;
        let a = Self::hash(xi, yi, seed);
        let b = Self::hash(xi + 1, yi, seed);
        let c = Self::hash(xi, yi + 1, seed);
        let d = Self::hash(xi + 1, yi + 1, seed);
        let u = Self::smooth(xf);
        let v = Self::smooth(yf);
        a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v
    }

    fn terrain_height(x: i32, y: i32) -> i32 {
        let mut amp = 1.0f32;
        let mut freq = 1.0f32;
        let mut sum = 0.0f32;
        let mut norm = 0.0f32;
        for oct in 0..4 {
            sum += amp * Self::value_noise(x as f32 * freq / 32.0, y as f32 * freq / 32.0, oct);
            norm += amp;
            amp *= 0.5;
            freq *= 2.0;
        }
        let n = sum / norm;
        let base = DEPTH as f32 * 2.0 / 3.0;
        (base + (n - 0.5) * 24.0).clamp(4.0, DEPTH as f32 - 6.0) as i32
    }
}

// --------------------------------------------------------------- mundo comun
// Las entidades (zombis, particulas) sirven para los dos motores del port: el
// Level de rd-132211/rd-132328 y el Nivel de la familia rd-160052..rd-161348.
pub trait Mundo {
    fn cajas(&self, bb: &BlockBox) -> Vec<BlockBox>;
}

impl Mundo for Level {
    fn cajas(&self, bb: &BlockBox) -> Vec<BlockBox> {
        self.cubes(bb)
    }
}

#[cfg(test)]
mod tests {
    use super::BlockBox;

    #[test]
    fn grown_extends_both_sides() {
        let b = BlockBox { x0: 1.0, y0: 2.0, z0: 3.0, x1: 4.0, y1: 5.0, z1: 6.0 };
        let g = b.grown(3.0, 2.0, 1.0);
        assert_eq!((g.x0, g.y0, g.z0), (-2.0, 0.0, 2.0));
        assert_eq!((g.x1, g.y1, g.z1), (7.0, 7.0, 7.0));
    }
}

// Level de la familia rd-160052 / rd-20090515: mundo de ids de tile con
// profundidades de luz por columna, generado con dos mapas Perlin de altura y
// uno de piedra.
//
// Ejes como el resto del port: x e y horizontales (y = fondo), z arriba.
// El array va indexado (z * HEIGHT + y) * WIDTH + x, igual que el original
// ((y * height + z) * width + x con y = vertical, z = fondo).

use super::perlin;
use super::tiles;
use crate::world::BlockBox;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::{Read, Write};

pub const WIDTH: usize = 256;
pub const HEIGHT: usize = 256;
pub const DEPTH: usize = 64;

pub struct Nivel {
    pub blocks: Vec<u8>,
    pub light: Vec<i32>,
    pub random: perlin::JavaRandom,
    // acumulador de Level.tick
    pub unprocessed: i32,
}

impl Nivel {
    pub fn new(seed: i64) -> Self {
        let mut n = Nivel {
            blocks: vec![0; WIDTH * HEIGHT * DEPTH],
            light: vec![0; WIDTH * HEIGHT],
            random: perlin::JavaRandom::new(seed),
            unprocessed: 0,
        };
        if n.load().is_none() {
            n.generar(seed);
        }
        n.calc_light(0, 0, WIDTH, HEIGHT);
        n.volcar_si_toca();
        n
    }

    fn idx(&self, x: usize, y: usize, z: usize) -> usize {
        // Java usa (alto * height + profundidad) * width + x; aqui x/y son
        // horizontales y z la altura, asi que el orden equivalente es este.
        (z * HEIGHT + y) * WIDTH + x
    }

    // generateMap() del original.
    fn generar(&mut self, seed: i64) {
        let sem = semillas(seed);
        let h1 = perlin::read(0, WIDTH, HEIGHT, sem[0]);
        let h2 = perlin::read(0, WIDTH, HEIGHT, sem[1]);
        let cf = perlin::read(1, WIDTH, HEIGHT, sem[2]);
        let rock = perlin::read(1, WIDTH, HEIGHT, sem[3]);
        let d = DEPTH as i32;
        for x in 0..WIDTH {
            for y in 0..HEIGHT {
                for z in 0..DEPTH {
                    let k = x + y * WIDTH;
                    let dh1 = h1[k];
                    let mut dh2 = h2[k];
                    let cfh = cf[k];
                    if cfh < 128 {
                        dh2 = dh1;
                    }
                    let mut dh = if dh2 > dh1 { dh2 } else { dh1 };
                    if dh2 <= dh1 {
                        dh2 = dh1;
                    }
                    let _ = dh2;
                    dh = dh / 8 + d / 3;
                    let mut rh = rock[k] / 8 + d / 3;
                    if rh > dh - 2 {
                        rh = dh - 2;
                    }
                    let zz = z as i32;
                    let id = if zz == dh {
                        tiles::CESPED
                    } else if zz < dh {
                        tiles::TIERRA
                    } else {
                        tiles::AIRE
                    };
                    let id = if zz <= rh { tiles::PIEDRA } else { id };
                    let i = self.idx(x, y, z);
                    self.blocks[i] = id;
                }
            }
        }
    }

    pub fn calc_light(&mut self, x0: usize, y0: usize, x1: usize, y1: usize) {
        for x in x0..(x0 + x1).min(WIDTH) {
            for y in y0..(y0 + y1).min(HEIGHT) {
                let mut z = DEPTH - 1;
                while z > 0 && !self.is_light_blocker(x, z, y) {
                    z -= 1;
                }
                self.light[x + y * WIDTH] = z as i32;
            }
        }
    }

    pub fn get(&self, x: i32, y: i32, z: i32) -> u8 {
        if x < 0
            || y < 0
            || z < 0
            || x as usize >= WIDTH
            || y as usize >= HEIGHT
            || z as usize >= DEPTH
        {
            return 0;
        }
        self.blocks[self.idx(x as usize, y as usize, z as usize)]
    }

    pub fn set(&mut self, x: i32, y: i32, z: i32, id: u8) -> bool {
        if x < 0
            || y < 0
            || z < 0
            || x as usize >= WIDTH
            || y as usize >= HEIGHT
            || z as usize >= DEPTH
        {
            return false;
        }
        let i = self.idx(x as usize, y as usize, z as usize);
        if self.blocks[i] == id {
            return false;
        }
        self.blocks[i] = id;
        self.calc_light(x as usize, y as usize, 1, 1);
        true
    }

    pub fn is_solid(&self, x: i32, y: i32, z: i32) -> bool {
        tiles::solido(self.get(x, y, z))
    }

    pub fn is_light_blocker(&self, x: usize, z: usize, y: usize) -> bool {
        tiles::bloquea_luz(self.get(x as i32, y as i32, z as i32))
    }

    pub fn is_lit(&self, x: i32, y: i32, z: i32) -> bool {
        if x < 0
            || y < 0
            || z < 0
            || x as usize >= WIDTH
            || y as usize >= HEIGHT
            || z as usize >= DEPTH
        {
            return true;
        }
        z >= self.light[x as usize + y as usize * WIDTH]
    }

    // Cajas de colisión de los bloques que tocan la zona dada (Level.getCubes).
    pub fn cubes(&self, aabb: &BlockBox) -> Vec<BlockBox> {
        let mut out = Vec::new();
        let x0 = (aabb.x0.floor() as i32).max(0);
        let x1 = ((aabb.x1 + 1.0).floor() as i32).min(WIDTH as i32);
        let y0 = (aabb.y0.floor() as i32).max(0);
        let y1 = ((aabb.y1 + 1.0).floor() as i32).min(HEIGHT as i32);
        let z0 = (aabb.z0.floor() as i32).max(0);
        let z1 = ((aabb.z1 + 1.0).floor() as i32).min(DEPTH as i32);
        for x in x0..x1 {
            for y in y0..y1 {
                for z in z0..z1 {
                    if self.is_solid(x, y, z) {
                        let (fx, fy, fz) = (x as f32, y as f32, z as f32);
                        out.push(BlockBox {
                            x0: fx,
                            x1: fx + 1.0,
                            y0: fy,
                            y1: fy + 1.0,
                            z0: fz,
                            z1: fz + 1.0,
                        });
                    }
                }
            }
        }
        out
    }
}

// ------------------------------------------------------------ mundo comun

impl crate::world::Mundo for Nivel {
    fn cajas(&self, bb: &BlockBox) -> Vec<BlockBox> {
        self.cubes(bb)
    }
}

// ------------------------------------------------ tick de los tiles (Level.tick)

impl Nivel {
    // Level.tick: acumula todos los bloques del mundo y procesa un tile al azar
    // por cada 400 acumulados (con 256x256x64 son ~10.000 por tick).
    pub fn tick(&mut self) {
        self.unprocessed += (WIDTH * HEIGHT * DEPTH) as i32;
        let mut ticks = self.unprocessed / 400;
        self.unprocessed -= ticks * 400;
        while ticks > 0 {
            let x = self.random.next_int(WIDTH as i32);
            let z = self.random.next_int(DEPTH as i32);
            let y = self.random.next_int(HEIGHT as i32);
            let id = self.get(x, y, z);
            tiles::tick(self, id, x, y, z);
            ticks -= 1;
        }
    }
}

// ------------------------------------------------------------ guardar/cargar
// El original escribe `level.dat` con GZIP del array de bloques tal cual. Aqui
// se escribe un gzip valido (bloques deflate "stored", sin comprimir) para no
// meter dependencias; al leerlo se aceptan esos mismos bloques.

fn crc32(datos: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &b in datos {
        crc ^= b as u32;
        for _ in 0..8 {
            let m = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xedb8_8320 & m);
        }
    }
    !crc
}

impl Nivel {
    // GZIP normal, como el DataOutputStream(GZIPOutputStream) del original:
    // cualquier lector de gzip (Java incluido) puede abrirlo.
    pub fn save(&self) -> std::io::Result<()> {
        let file = std::fs::File::create("level.dat")?;
        let mut gz = GzEncoder::new(std::io::BufWriter::new(file), Compression::default());
        gz.write_all(&self.blocks)?;
        gz.finish()?.flush()
    }

    // Lee cualquier level.dat gzip (tambien los del Java original).
    pub fn load(&mut self) -> Option<()> {
        let file = std::fs::File::open("level.dat").ok()?;
        let mut data = Vec::with_capacity(self.blocks.len());
        GzDecoder::new(std::io::BufReader::new(file))
            .read_to_end(&mut data)
            .ok()?;
        if data.len() != self.blocks.len() {
            return None;
        }
        self.blocks.copy_from_slice(&data);
        Some(())
    }
}

// Semillas de los cuatro filtros. El original crea un Random distinto por filtro
// (semilla del reloj), asi que su mundo cambia en cada arranque; con RD_SEEDS se
// fijan para el careo contra el Java, que se parchea con las mismas.
fn semillas(seed: i64) -> [i64; 4] {
    if let Ok(v) = std::env::var("RD_SEEDS") {
        let n: Vec<i64> = v.split(',').filter_map(|t| t.trim().parse().ok()).collect();
        if n.len() == 4 {
            return [n[0], n[1], n[2], n[3]];
        }
    }
    [
        seed,
        seed ^ 0x1234_5678,
        seed ^ 0x2468_ACE0,
        seed ^ 0x0F0F_0F0F,
    ]
}

impl Nivel {
    // RD_DUMP=1 escribe bloques y luces como los vuelca el Java parcheado.
    fn volcar_si_toca(&self) {
        if std::env::var("RD_DUMP").is_err() {
            return;
        }
        let _ = std::fs::write("bloques_port.raw", &self.blocks);
        let mut luces = Vec::with_capacity(self.light.len() * 4);
        for l in &self.light {
            luces.extend_from_slice(&l.to_be_bytes());
        }
        let _ = std::fs::write("luces_port.raw", &luces);
    }
}

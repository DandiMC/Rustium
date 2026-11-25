// Level de la familia rd-160052 / rd-20090515: mundo de ids de tile con
// profundidades de luz por columna, generado con dos mapas Perlin de altura y
// uno de piedra.
//
// Ejes como el resto del port: x e y horizontales (y = fondo), z arriba.
// El array va indexado (z * HEIGHT + y) * WIDTH + x, igual que el original
// ((y * height + z) * width + x con y = vertical, z = fondo).

use super::noisemap;
use super::perlin;
use super::tiles;
use crate::world::BlockBox;

pub const WIDTH: usize = 256;
pub const HEIGHT: usize = 256;
pub const DEPTH: usize = 64;
/// Nivel del mar de c0.0.12a: el mundo queda rodeado de agua y las islas
/// sobresalen por encima de esta cota.
pub const SEA_LEVEL: i32 = (DEPTH as i32 * 5) / 8;

pub struct Nivel {
    pub blocks: Vec<u8>,
    pub light: Vec<i32>,
    pub random: perlin::JavaRandom,
    // acumulador de Level.tick
    pub unprocessed: i32,
    // Columnas (x, fondo) tocadas desde la ultima reconstruccion. Level avisaba
    // a LevelRenderer por cada setTile; el port necesita lo mismo para los
    // cambios que hace el tick (crecimiento de hierba, arbustos).
    pub dirty: Vec<(i32, i32)>,
    // Bloques de agua/lava pendientes de extenderse. Solo se llena cuando el
    // jugador coloca un liquido, no con el oceano generado, para que el mundo
    // no se inunde entero nada mas empezar.
    pub liquidos: Vec<(i32, i32, i32)>,
}

impl Nivel {
    pub fn new(seed: i64) -> Self {
        let mut n = Nivel {
            blocks: vec![0; WIDTH * HEIGHT * DEPTH],
            light: vec![0; WIDTH * HEIGHT],
            random: perlin::JavaRandom::new(seed),
            unprocessed: 0,
            dirty: Vec::new(),
            liquidos: Vec::new(),
        };
        if n.load().is_none() {
            n.generar(seed);
        }
        n.calc_light(0, 0, WIDTH, HEIGHT);
        n.volcar_si_toca();
        n
    }

    // Classic c0.0.11a: Level encarga el mapa a LevelGen, que en vez de Perlin
    // usa cuatro NoiseMap y cava cuevas con un Random propio.
    pub fn new_clasico(seed: i64) -> Self {
        let mut n = Nivel {
            blocks: vec![0; WIDTH * HEIGHT * DEPTH],
            light: vec![0; WIDTH * HEIGHT],
            random: perlin::JavaRandom::new(seed),
            unprocessed: 0,
            dirty: Vec::new(),
            liquidos: Vec::new(),
        };
        if n.load().is_none() {
            n.generar_clasico(seed);
        }
        n.calc_light(0, 0, WIDTH, HEIGHT);
        n.volcar_si_toca();
        n
    }

    /// c0.0.12a: el mundo tiene una capa de bedrock, un oceano alrededor de las
    /// islas y lagos de lava. Es el primer Classic con liquidos.
    pub fn new_clasico_012(seed: i64) -> Self {
        let mut n = Nivel {
            blocks: vec![0; WIDTH * HEIGHT * DEPTH],
            light: vec![0; WIDTH * HEIGHT],
            random: perlin::JavaRandom::new(seed),
            unprocessed: 0,
            dirty: Vec::new(),
            liquidos: Vec::new(),
        };
        if n.load().is_none() {
            n.generar_clasico_012(seed);
        }
        // El oceano y los lagos generados son estaticos; solo los liquidos que
        // coloca el jugador entran en la cola de flujo.
        n.liquidos.clear();
        n.calc_light(0, 0, WIDTH, HEIGHT);
        n.volcar_si_toca();
        n
    }

    // LevelGen.generateMap de 0.0.12a. La altura y las cuevas salen del mismo
    // NoiseMap que 0.0.11a; encima se anaden el oceano, la capa de bedrock y
    // algunos lagos de lava. No usa ningun asset de Mojang.
    fn generar_clasico_012(&mut self, seed: i64) {
        let sem = semillas(seed);
        let h1 = noisemap::read(0, WIDTH, HEIGHT, sem[0]);
        let h2 = noisemap::read(0, WIDTH, HEIGHT, sem[1]);
        let cf = noisemap::read(1, WIDTH, HEIGHT, sem[2]);
        let rock = noisemap::read(1, WIDTH, HEIGHT, sem[3]);
        let lava = noisemap::read(2, WIDTH, HEIGHT, seed ^ 0x012A_5EED);
        let mut superficie = vec![0i32; WIDTH * HEIGHT];
        let d = DEPTH as i32;
        let mar = SEA_LEVEL;
        for x in 0..WIDTH {
            for y in 0..HEIGHT {
                for z in 0..DEPTH {
                    let k = x + y * WIDTH;
                    let dh1 = h1[k];
                    let mut dh2 = h2[k];
                    if cf[k] < 128 {
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
                    superficie[k] = dh;
                    let zz = z as i32;
                    let mut id = if zz == 0 {
                        tiles::BEDROCK
                    } else if zz <= rh {
                        tiles::PIEDRA
                    } else if zz < dh {
                        tiles::TIERRA
                    } else if zz == dh {
                        if dh >= mar { tiles::CESPED } else { tiles::TIERRA }
                    } else if zz <= mar {
                        tiles::AGUA
                    } else {
                        tiles::AIRE
                    };
                    // Lagos de lava pequenos en la superficie. Es raro a
                    // proposito: no queremos convertir el mapa en un infierno.
                    if id == tiles::CESPED && lava[k] > 252 {
                        id = tiles::LAVA;
                    }
                    let i = self.idx(x, y, z);
                    self.blocks[i] = id;
                }
            }
        }
        self.cavar_cuevas(seed);
        self.anadir_lagos_lava(seed, &superficie);
        self.anadir_lava_en_cuevas(seed);
    }

    // Lagos de lava de superficie. El primer 0.0.12a tenia lagos, no solo
    // bloques sueltos; estos son pequenos y se colocan sobre tierra firme.
    fn anadir_lagos_lava(&mut self, seed: i64, superficie: &[i32]) {
        let mut rnd = perlin::JavaRandom::new(seed ^ 0x1A2A_910B);
        let (w, h) = (WIDTH as i32, HEIGHT as i32);
        for _ in 0..16 {
            let cx = 4 + rnd.next_int(w - 8);
            let cy = 4 + rnd.next_int(h - 8);
            let k = cx as usize + cy as usize * WIDTH;
            let alto = superficie[k].clamp(SEA_LEVEL + 2, DEPTH as i32 - 3);
            for dx in -2..=2 {
                for dy in -2..=2 {
                    let x = cx + dx;
                    let y = cy + dy;
                    if x < 1 || y < 1 || x >= w - 1 || y >= h - 1 {
                        continue;
                    }
                    let k = x as usize + y as usize * WIDTH;
                    let z = superficie[k].clamp(alto, DEPTH as i32 - 2);
                    self.set(x, y, z, tiles::LAVA);
                    if z + 1 < DEPTH as i32 {
                        self.set(x, y, z + 1, tiles::AIRE);
                    }
                }
            }
        }
    }

    // Algunas cuevas por debajo del nivel del mar se llenan de lava. Sirve para
    // que el jugador encuentre lava tambien al explorar, no solo en la superficie.
    fn anadir_lava_en_cuevas(&mut self, seed: i64) {
        let mut rnd = perlin::JavaRandom::new(seed ^ 0x00C0_12A5);
        let (w, h) = (WIDTH as i32, HEIGHT as i32);
        for _ in 0..(w * h / 900) {
            let x = rnd.next_int(w);
            let y = rnd.next_int(h);
            let z = 1 + rnd.next_int((SEA_LEVEL - 2).max(1));
            let i = self.idx(x as usize, y as usize, z as usize);
            if self.blocks[i] == tiles::AIRE
                && self.get(x, y, z - 1) != tiles::AGUA
                && self.get(x, y, z - 1) != tiles::LAVA
            {
                self.blocks[i] = tiles::LAVA;
            }
        }
    }

    // LevelGen.generateMap de c0.0.11a. Identico al de la familia B salvo los
    // cuatro NoiseMap y las cuevas (LevelGen se las hace aqui, no en Level).
    fn generar_clasico(&mut self, seed: i64) {
        let sem = semillas(seed);
        let h1 = noisemap::read(0, WIDTH, HEIGHT, sem[0]);
        let h2 = noisemap::read(0, WIDTH, HEIGHT, sem[1]);
        let cf = noisemap::read(1, WIDTH, HEIGHT, sem[2]);
        let rock = noisemap::read(1, WIDTH, HEIGHT, sem[3]);
        let d = DEPTH as i32;
        for x in 0..WIDTH {
            for y in 0..HEIGHT {
                for z in 0..DEPTH {
                    let k = x + y * WIDTH;
                    let dh1 = h1[k];
                    let mut dh2 = h2[k];
                    if cf[k] < 128 {
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
        self.cavar_cuevas(seed);
    }

    // Segunda mitad de LevelGen.generateMap: gusanos de piedra que se comen la
    // roca. Los 256 tuneles (w*h*d/256/64) salen de un Random que en el original
    // se crea con la hora; aqui va con semilla para que el mundo se reproduzca.
    fn cavar_cuevas(&mut self, seed: i64) {
        let mut random = perlin::JavaRandom::new(seed ^ 0xC0FFEE);
        let (w, h, d) = (WIDTH as i32, HEIGHT as i32, DEPTH as i32);
        let count = w * h * d / 256 / 64;
        for _ in 0..count {
            let mut x2 = random.next_float() * w as f32;
            let mut y = random.next_float() * d as f32;
            let mut z = random.next_float() * h as f32;
            let length = (random.next_float() + random.next_float() * 150.0) as i32;
            let mut dir1 = random.next_float() * std::f32::consts::PI * 2.0;
            let mut dira1 = 0.0f32;
            let mut dir2 = random.next_float() * std::f32::consts::PI * 2.0;
            let mut dira2 = 0.0f32;
            for l in 0..length {
                x2 = (x2 as f64 + (dir1 as f64).sin() * (dir2 as f64).cos()) as f32;
                z = (z as f64 + (dir1 as f64).cos() * (dir2 as f64).cos()) as f32;
                y = (y as f64 + (dir2 as f64).sin()) as f32;
                dir1 += dira1 * 0.2;
                dira1 *= 0.9;
                dira1 += random.next_float() - random.next_float();
                dir2 += dira2 * 0.5;
                dir2 *= 0.5;
                dira2 *= 0.9;
                dira2 += random.next_float() - random.next_float();
                let size = ((l as f64 * std::f64::consts::PI / length as f64).sin() * 2.5 + 1.0) as f32;
                let mut xx = (x2 - size) as i32;
                while xx as f32 <= x2 + size {
                    let mut yy = (y - size) as i32;
                    while yy as f32 <= y + size {
                        let mut zz = (z - size) as i32;
                        while zz as f32 <= z + size {
                            let xd = xx as f32 - x2;
                            let yd = yy as f32 - y;
                            let zd = zz as f32 - z;
                            let dd = xd * xd + yd * yd * 2.0 + zd * zd;
                            if dd < size * size
                                && xx >= 1
                                && yy >= 1
                                && zz >= 1
                                && xx < w - 1
                                && yy < d - 1
                                && zz < h - 1
                            {
                                let i = self.idx(xx as usize, zz as usize, yy as usize);
                                if self.blocks[i] == tiles::PIEDRA {
                                    self.blocks[i] = tiles::AIRE;
                                }
                            }
                            zz += 1;
                        }
                        yy += 1;
                    }
                    xx += 1;
                }
            }
        }
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
        self.dirty.push((x, y));
        if tiles::es_liquido(id) && self.liquidos.len() < 8192 {
            self.liquidos.push((x, y, z));
        }
        self.calc_light(x as usize, y as usize, 1, 1);
        true
    }

    // LevelListener.tileChanged: el juego consume las columnas cambiadas y
    // reconstruye sus chunks (y los vecinos, por el cambio de luz).
    pub fn tomar_cambios(&mut self) -> Vec<(i32, i32)> {
        std::mem::take(&mut self.dirty)
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
        self.fluir_liquidos(64);
    }

    /// Riega/expande hasta 64 bloques pendientes por tick. El agua y la lava
    /// caen primero y, si no pueden, buscan un hueco al lado. Cuando se
    /// encuentran, el agua convierte la lava en piedra, como en 0.0.12a.
    fn fluir_liquidos(&mut self, max: usize) {
        let mut hechos = 0usize;
        let mut i = 0usize;
        while i < self.liquidos.len() && hechos < max {
            let (x, y, z) = self.liquidos[i];
            i += 1;
            let id = self.get(x, y, z);
            if !tiles::es_liquido(id) {
                continue;
            }
            let mut destino = None;
            let abajo = self.get(x, y, z - 1);
            if abajo == tiles::AIRE || (tiles::es_liquido(abajo) && abajo != id) {
                destino = Some((x, y, z - 1));
            } else {
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let (nx, ny) = (x + dx, y + dy);
                    let vecino = self.get(nx, ny, z);
                    if vecino == tiles::AIRE || (tiles::es_liquido(vecino) && vecino != id) {
                        destino = Some((nx, ny, z));
                        break;
                    }
                }
            }
            if let Some((nx, ny, nz)) = destino {
                let otro = self.get(nx, ny, nz);
                if tiles::es_liquido(otro) && otro != id {
                    self.set(nx, ny, nz, tiles::PIEDRA);
                } else if otro == tiles::AIRE {
                    self.set(nx, ny, nz, id);
                }
            }
            hechos += 1;
        }
        if i > 0 {
            self.liquidos.drain(0..i.min(self.liquidos.len()));
        }
        // Si el jugador vacia un oceano entero, no dejamos una cola sin fin.
        if self.liquidos.len() > 8192 {
            self.liquidos.truncate(8192);
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
        crate::world::save_gzip_atomic(std::path::Path::new("level.dat"), &self.blocks)
    }

    // Lee cualquier level.dat gzip (tambien los del Java original). Si el
    // principal esta truncado, prueba el temporal y luego la copia anterior.
    pub fn load(&mut self) -> Option<()> {
        let data = crate::world::load_gzip_with_backup(
            std::path::Path::new("level.dat"),
            self.blocks.len(),
        )?;
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

#[cfg(test)]
mod tests_012 {
    use super::*;

    fn generar(seed: i64) -> Nivel {
        let mut n = Nivel {
            blocks: vec![0; WIDTH * HEIGHT * DEPTH],
            light: vec![0; WIDTH * HEIGHT],
            random: perlin::JavaRandom::new(seed),
            unprocessed: 0,
            dirty: Vec::new(),
            liquidos: Vec::new(),
        };
        n.generar_clasico_012(seed);
        n
    }

    #[test]
    fn el_agua_colocada_se_extiende() {
        let mut n = Nivel {
            blocks: vec![0; WIDTH * HEIGHT * DEPTH],
            light: vec![0; WIDTH * HEIGHT],
            random: perlin::JavaRandom::new(1),
            unprocessed: 0,
            dirty: Vec::new(),
            liquidos: Vec::new(),
        };
        for x in 8..13 {
            for y in 8..13 {
                n.set(x, y, 10, tiles::PIEDRA);
            }
        }
        n.set(10, 10, 11, tiles::AGUA);
        assert!(!n.liquidos.is_empty());
        n.fluir_liquidos(8);
        let agua = (8..13)
            .flat_map(|x| (8..13).map(move |y| (x, y)))
            .filter(|(x, y)| n.get(*x, *y, 11) == tiles::AGUA)
            .count();
        assert!(agua >= 2, "el agua no se extendio: {agua}");
    }

    #[test]
    fn el_012_tiene_bedrock_agua_y_lava() {
        let n = generar(12345);
        let mut agua = 0usize;
        let mut lava = 0usize;
        let mut agua_pos = None;
        let mut lava_pos = None;
        let mut agua_playa = Vec::new();
        let mut lava_aire = Vec::new();
        for x in 0..WIDTH {
            for y in 0..HEIGHT {
                assert_eq!(n.get(x as i32, y as i32, 0), tiles::BEDROCK);
                for z in 0..DEPTH {
                    match n.get(x as i32, y as i32, z as i32) {
                        tiles::AGUA => {
                            agua += 1;
                            if agua_pos.is_none()
                                && z + 1 < DEPTH
                                && n.get(x as i32, y as i32, z as i32 + 1) == tiles::AIRE
                            {
                                agua_pos = Some((x, y, z));
                            }
                            if agua_playa.len() < 8
                                && z + 1 < DEPTH
                                && n.get(x as i32, y as i32, z as i32 + 1) == tiles::AIRE
                                && (x > 20 && x < 236 && y > 20 && y < 236)
                            {
                                agua_playa.push((x, y, z));
                            }
                        }
                        tiles::LAVA => {
                            lava += 1;
                            if lava_pos.is_none() {
                                lava_pos = Some((x, y, z));
                            }
                            if lava_aire.len() < 8
                                && z + 1 < DEPTH
                                && n.get(x as i32, y as i32, z as i32 + 1) == tiles::AIRE
                            {
                                lava_aire.push((x, y, z));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        println!("012 agua_playa={agua_playa:?} lava_aire={lava_aire:?}");
        println!("012 agua={agua} lava={lava} agua_pos={agua_pos:?} lava_pos={lava_pos:?}");
        assert!(agua > 10_000, "muy poca agua: {agua}");
        assert!(lava > 100, "muy poca lava: {lava}");
        assert!(agua_pos.is_some());
        assert!(lava_pos.is_some());
    }
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

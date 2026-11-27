// Level de la familia rd-160052 / rd-20090515 y de Classic: mundo de ids de
// tile con profundidades de luz por columna. Los ejes del port son x e y
// horizontales (y = fondo) y z arriba; el array va indexado
// (z * alto + y) * ancho + x, igual que el original
// ((y * height + z) * width + x con y = vertical, z = fondo).
//
// Hasta 0.0.12a_03 el level.dat era solo el array de bloques comprimido con
// gzip. Desde 0.0.13a lleva una cabecera con nombre, autor, fecha y dimensiones;
// 0.0.13a-dev usa un mundo plano de 32x512x64. El port acepta los dos formatos.

use super::noisemap;
use super::perlin;
use super::tiles;
use crate::world::BlockBox;
use std::io;
use std::path::Path;

/// Dimensiones por defecto de Classic (x, fondo horizontal, altura).
pub const WIDTH: usize = 256;
pub const HEIGHT: usize = 256;
pub const DEPTH: usize = 64;
/// Nivel del mar de c0.0.12a: el mundo queda rodeado de agua y las islas
/// sobresalen por encima de esta cota.
pub const SEA_LEVEL: i32 = (DEPTH as i32 * 5) / 8;

/// Formato de level.dat. `Crudo` es el de rd-132211..0.0.12a_03; `Cabecera13a`
/// es el que introdujo 0.0.13a y que también usa 0.0.13a_03.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Formato {
    Crudo,
    Cabecera13a,
}

/// Forma de la copa de los arboles segun la build.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Copa {
    /// 0.0.14a base: 5x5 con las esquinas recortadas y remate 3x3.
    Cuadrada,
    /// 0.0.14a_08: el remate pasa a forma de cruz.
    Cruz,
    /// 0.0.15a: roble pequeno, con dos capas de 5x5 y remate 3x3 con cruz.
    Roble,
}

// LevelIO: MAGIC_NUMBER = 656127880, CURRENT_VERSION = 1.
const MAGIC_LEVEL: i32 = 656_127_880;
const VERSION_LEVEL: u8 = 1;

pub struct Nivel {
    /// x: ancho.
    pub ancho: usize,
    /// z horizontal (el `height` del Level de Java).
    pub alto: usize,
    /// y vertical (el `depth` del Level de Java).
    pub fondo: usize,
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
    /// 0.0.15a: la lava espera un tick de cada dos antes de extenderse.
    pub lava_lenta: bool,
    /// 0.0.20a: las flores y setas se marchitan a oscuras. 0.0.24 lo quita.
    pub plantas_no_decauen: bool,
    /// Ticks transcurridos, para el flujo lento de la lava.
    pub ticks: u64,
    pub formato: Formato,
    pub nombre: String,
    pub creador: String,
    pub creado: i64,
}

fn ahora_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

impl Nivel {
    fn vacio(seed: i64, ancho: usize, alto: usize, fondo: usize, formato: Formato) -> Self {
        Nivel {
            ancho,
            alto,
            fondo,
            blocks: vec![0; ancho * alto * fondo],
            light: vec![0; ancho * alto],
            random: perlin::JavaRandom::new(seed),
            unprocessed: 0,
            dirty: Vec::new(),
            liquidos: Vec::new(),
            lava_lenta: false,
            plantas_no_decauen: false,
            ticks: 0,
            formato,
            nombre: "A Nice World".to_string(),
            creador: "noname".to_string(),
            creado: ahora_millis(),
        }
    }

    /// Nivel vacío con dimensiones arbitrarias. Lo usan los tests y la carga de
    /// un level.dat con cabecera, antes de decidir su tamaño definitivo.
    pub fn nuevo_vacio(seed: i64, ancho: usize, alto: usize, fondo: usize) -> Self {
        Self::vacio(seed, ancho, alto, fondo, Formato::Crudo)
    }

    pub fn new(seed: i64) -> Self {
        let mut n = Self::vacio(seed, WIDTH, HEIGHT, DEPTH, Formato::Crudo);
        if n.load().is_none() {
            n.generar(seed);
        }
        n.calc_light(0, 0, n.ancho, n.alto);
        n.volcar_si_toca();
        n
    }

    // Classic c0.0.11a: Level encarga el mapa a LevelGen, que en vez de Perlin
    // usa cuatro NoiseMap y cava cuevas con un Random propio.
    pub fn new_clasico(seed: i64) -> Self {
        let mut n = Self::vacio(seed, WIDTH, HEIGHT, DEPTH, Formato::Crudo);
        if n.load().is_none() {
            n.generar_clasico(seed);
        }
        n.calc_light(0, 0, n.ancho, n.alto);
        n.volcar_si_toca();
        n
    }

    /// c0.0.12a: el mundo tiene una capa de bedrock, un oceano alrededor de las
    /// islas y lagos de lava. Es el primer Classic con liquidos.
    pub fn new_clasico_012(seed: i64) -> Self {
        Self::new_clasico_012_con(seed, true)
    }

    /// c0.0.13a_03 es oficial y usa el level.dat con cabecera de 0.0.13a.
    pub fn new_clasico_013a03(seed: i64) -> Self {
        let mut n = Self::vacio(seed, WIDTH, HEIGHT, DEPTH, Formato::Cabecera13a);
        if n.load().is_none() {
            n.generar_clasico_012_con(seed, false);
        }
        n.liquidos.clear();
        n.calc_light(0, 0, n.ancho, n.alto);
        n.volcar_si_toca();
        n
    }

    /// c0.0.14a: el salto de familia. Sigue el mundo clasico de 256x256x64,
    /// pero el generador anade playas de arena y grava, menas de carbon,
    /// hierro y oro, y arboles de tronco y hojas. La arena y la grava caen.
    pub fn new_clasico_014(seed: i64) -> Self {
        Self::new_clasico_014_con(seed, Copa::Cuadrada, false, false)
    }

    /// c0.0.14a_08: ultima build de desarrollo de la familia, anterior al
    /// test multijugador. La copa de los arboles pasa a forma de cruz; el
    /// resto del mundo es el de 0.0.14a.
    pub fn new_clasico_014a08(seed: i64) -> Self {
        Self::new_clasico_014_con(seed, Copa::Cruz, false, false)
    }

    /// 0.0.15a: primer test multijugador. La grava ya solo aparece bajo el
    /// agua, los arboles cambian a la copa de roble pequeno y la lava fluye
    /// mas despacio.
    pub fn new_clasico_015(seed: i64) -> Self {
        Self::new_clasico_014_con(seed, Copa::Roble, true, true)
    }

    /// 0.0.19a: el mundo es el mismo de 0.0.15a; lo nuevo son los bloques
    /// (cristal, esponja) y la barra rapida.
    pub fn new_clasico_019(seed: i64) -> Self {
        Self::new_clasico_015(seed)
    }

    /// 0.0.20a: sobre el mundo de 0.0.15a crecen dienteleones y rosas en la
    /// hierba y setas en las cuevas. Las plantas se marchitan a oscuras hasta
    /// 0.0.24.
    pub fn new_clasico_020(seed: i64) -> Self {
        // Si ya hay un level.dat no se vuelven a sembrar flores y setas: si no,
        // cada arranque anadiria plantas nuevas al mismo mundo.
        let existente = Path::new("level.dat").exists();
        let mut n = Self::new_clasico_015(seed);
        n.plantas_no_decauen = false;
        if !existente {
            n.sembrar_plantas(seed);
            n.volcar_si_toca();
        }
        n
    }

    fn new_clasico_014_con(
        seed: i64,
        copa: Copa,
        grava_sumergida: bool,
        lava_lenta: bool,
    ) -> Self {
        let mut n = Self::vacio(seed, WIDTH, HEIGHT, DEPTH, Formato::Cabecera13a);
        n.lava_lenta = lava_lenta;
        if n.load().is_none() {
            n.generar_clasico_014_con(seed, copa, grava_sumergida);
        }
        n.liquidos.clear();
        n.calc_light(0, 0, n.ancho, n.alto);
        n.volcar_si_toca();
        n
    }

    /// c0.0.13a-dev, la build archivada en el launcher: mundo completamente
    /// plano de 32x512x64 (x, fondo, altura). `LevelGen` la genera con hierba en
    /// la cota depth/2, tierra debajo, piedra en depth/3 y tuneles en la roca.
    /// No hay lagos de lava en la superficie; la lava solo aparece en cuevas.
    pub fn new_013a_dev(seed: i64) -> Self {
        let mut n = Self::vacio(seed, 32, 512, 64, Formato::Cabecera13a);
        if n.load().is_none() {
            n.generar_013a_dev(seed);
        }
        n.liquidos.clear();
        n.calc_light(0, 0, n.ancho, n.alto);
        n.volcar_si_toca();
        n
    }

    fn new_clasico_012_con(seed: i64, lava_superficie: bool) -> Self {
        let mut n = Self::vacio(seed, WIDTH, HEIGHT, DEPTH, Formato::Crudo);
        if n.load().is_none() {
            n.generar_clasico_012_con(seed, lava_superficie);
        }
        // El oceano y los lagos generados son estaticos; solo los liquidos que
        // coloca el jugador entran en la cola de flujo.
        n.liquidos.clear();
        n.calc_light(0, 0, n.ancho, n.alto);
        n.volcar_si_toca();
        n
    }

    // LevelGen.generateMap de 0.0.12a. La altura y las cuevas salen del mismo
    // NoiseMap que 0.0.11a; encima se anaden el oceano, la capa de bedrock y
    // algunos lagos de lava. No usa ningun asset de Mojang.
    fn generar_clasico_012(&mut self, seed: i64) {
        self.generar_clasico_012_con(seed, true);
    }

    fn generar_clasico_012_con(&mut self, seed: i64, lava_superficie: bool) {
        let (w, h, d) = (self.ancho, self.alto, self.fondo);
        let sem = semillas(seed);
        let h1 = noisemap::read(0, w, h, sem[0]);
        let h2 = noisemap::read(0, w, h, sem[1]);
        let cf = noisemap::read(1, w, h, sem[2]);
        let rock = noisemap::read(1, w, h, sem[3]);
        let lava = noisemap::read(2, w, h, seed ^ 0x012A_5EED);
        let mut superficie = vec![0i32; w * h];
        let d = d as i32;
        let mar = SEA_LEVEL;
        for x in 0..w {
            for y in 0..h {
                for z in 0..d as usize {
                    let k = x + y * w;
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
                    // En 0.0.13a_03 ya no aparecen por encima del nivel del mar.
                    if lava_superficie && id == tiles::CESPED && lava[k] > 252 {
                        id = tiles::LAVA;
                    }
                    let i = self.idx(x, y, z);
                    self.blocks[i] = id;
                }
            }
        }
        self.cavar_cuevas(seed);
        if lava_superficie {
            self.anadir_lagos_lava(seed, &superficie);
        }
        self.anadir_lava_en_cuevas(seed);
    }

    /// Generador de 0.0.14a. La wiki documenta el terreno nuevo con playas,
    /// menas y arboles, pero no su algoritmo exacto (el jar esta perdido), asi
    /// que se parte del relieve y las cuevas de 0.0.12a y se decoran encima.
    fn generar_clasico_014_con(&mut self, seed: i64, copa: Copa, grava_sumergida: bool) {
        self.generar_clasico_012_con(seed, false);
        self.sembrar_playas(seed, grava_sumergida);
        self.sembrar_menas(seed);
        self.sembrar_arboles(seed, copa);
    }

    /// Flores y setas de 0.0.20a. Las flores buscan una columna de hierba con
    /// aire encima; las setas prefieren cuevas y zonas sin cielo, que es donde
    /// el original las deja crecer. Usa un Random propio para no alterar el
    /// orden de las menas ni de los arboles.
    fn sembrar_plantas(&mut self, seed: i64) {
        let mut random = perlin::JavaRandom::new(seed ^ 0x5EED_2020);
        let (w, h, d) = (self.ancho as i32, self.alto as i32, self.fondo as i32);
        // Dienteleones y rosas: una de cada seis columnas de hierba con aire.
        for _ in 0..(w * h / 6).max(1) {
            let x = random.next_int(w);
            let y = random.next_int(h);
            let Some(superficie) = self.altura_superficie(x, y) else {
                continue;
            };
            if self.get(x, y, superficie) == tiles::CESPED
                && self.get(x, y, superficie + 1) == tiles::AIRE
            {
                let flor = if random.next_int(2) == 0 {
                    tiles::DENTELEON
                } else {
                    tiles::ROSA
                };
                self.set(x, y, superficie + 1, flor);
            }
        }
        // Setas: buscan un hueco a oscuras con suelo solido debajo.
        for _ in 0..(w * h / 8).max(1) {
            let x = random.next_int(w);
            let y = random.next_int(h);
            let z = 1 + random.next_int((d - 2).max(1));
            let abajo = self.get(x, y, z - 1);
            let suelo = matches!(
                abajo,
                tiles::TIERRA | tiles::CESPED | tiles::PIEDRA | tiles::GRAVA
            );
            if suelo && self.get(x, y, z) == tiles::AIRE && !self.is_lit(x, y, z) {
                let seta = if random.next_int(4) == 0 {
                    tiles::CHAMPINON_ROJO
                } else {
                    tiles::CHAMPINON_MARRON
                };
                self.set(x, y, z, seta);
            }
        }
    }

    /// 0.0.19a: la esponja chupa el agua de un cubo de 5x5x5 y desaparece. Si
    /// no habia agua se queda colocada hasta que el tick la seque.
    pub fn absorber_esponja(&mut self, x: i32, y: i32, z: i32) -> usize {
        let mut absorbidos = 0usize;
        for dx in -2..=2 {
            for dy in -2..=2 {
                for dz in -2..=2 {
                    if self.get(x + dx, y + dy, z + dz) == tiles::AGUA {
                        self.set(x + dx, y + dy, z + dz, tiles::AIRE);
                        absorbidos += 1;
                    }
                }
            }
        }
        if absorbidos > 0 {
            self.set(x, y, z, tiles::AIRE);
        }
        absorbidos
    }

    fn altura_superficie(&self, x: i32, y: i32) -> Option<i32> {
        let mut z = self.fondo as i32 - 1;
        while z >= 0 {
            let id = self.get(x, y, z);
            if id != tiles::AIRE && !tiles::es_liquido(id) {
                return Some(z);
            }
            z -= 1;
        }
        None
    }

    /// Playas: la arena aparece en la orilla, a la altura del mar. La grava
    /// forma algunos bancos sueltos, como en las capturas de la version.
    fn sembrar_playas(&mut self, seed: i64, grava_sumergida: bool) {
        let (w, h) = (self.ancho as i32, self.alto as i32);
        for x in 0..w {
            for y in 0..h {
                let Some(z) = self.altura_superficie(x, y) else { continue };
                if self.get(x, y, z) != tiles::CESPED {
                    continue;
                }
                if z <= SEA_LEVEL + 1 && z >= SEA_LEVEL - 2 {
                    let i = self.idx(x as usize, y as usize, z as usize);
                    self.blocks[i] = tiles::ARENA;
                    if z > 0 && self.get(x, y, z - 1) == tiles::TIERRA {
                        let j = self.idx(x as usize, y as usize, (z - 1) as usize);
                        self.blocks[j] = tiles::ARENA;
                    }
                }
            }
        }
        let mut rnd = perlin::JavaRandom::new(seed ^ 0x0E14_A9E5);
        for _ in 0..28 {
            let cx = 4 + rnd.next_int(w - 8);
            let cy = 4 + rnd.next_int(h - 8);
            let radio = 1 + rnd.next_int(4);
            for dx in -radio..=radio {
                for dy in -radio..=radio {
                    if dx * dx + dy * dy > radio * radio {
                        continue;
                    }
                    let (x, y) = (cx + dx, cy + dy);
                    let Some(z) = self.altura_superficie(x, y) else { continue };
                    if self.get(x, y, z) == tiles::CESPED {
                        // 0.0.15a: la grava de superficie desaparece; solo se
                        // queda la que el mar cubre.
                        if grava_sumergida && z > SEA_LEVEL {
                            continue;
                        }
                        let i = self.idx(x as usize, y as usize, z as usize);
                        self.blocks[i] = tiles::GRAVA;
                    }
                }
            }
        }
    }

    /// Menas de 10-30 bloques, como las documenta la wiki. El carbon sale a
    /// cualquier profundidad, el hierro en la mitad y el oro cerca del fondo.
    fn sembrar_menas(&mut self, seed: i64) {
        let mut rnd = perlin::JavaRandom::new(seed ^ 0x014A_0E5);
        let (w, h, d) = (self.ancho as i32, self.alto as i32, self.fondo as i32);
        let capas: [(u8, usize, i32, i32); 3] = [
            (tiles::MENA_CARBON, 70, 24, d - 3),
            (tiles::MENA_HIERRO, 50, 8, d * 2 / 3),
            (tiles::MENA_ORO, 18, 2, d / 2),
        ];
        for (id, copias, zmin, zmax) in capas {
            for _ in 0..copias {
                let mut x = rnd.next_int(w);
                let mut y = rnd.next_int(h);
                let mut z = zmin + rnd.next_int((zmax - zmin).max(1));
                let n = 10 + rnd.next_int(21);
                for _ in 0..n {
                    if self.get(x, y, z) == tiles::PIEDRA {
                        let i = self.idx(x as usize, y as usize, z as usize);
                        self.blocks[i] = id;
                    }
                    x = (x + rnd.next_int(3) - 1).clamp(0, w - 1);
                    y = (y + rnd.next_int(3) - 1).clamp(0, h - 1);
                    z = (z + rnd.next_int(3) - 1).clamp(0, d - 1);
                }
            }
        }
    }

    /// Arboles de roble: tronco de 4-6 y copa de hojas, sobre cesped por
    /// encima del nivel del mar. No se plantan si no caben en el mundo.
    fn sembrar_arboles(&mut self, seed: i64, copa: Copa) {
        let mut rnd = perlin::JavaRandom::new(seed ^ 0x00A7_7E55);
        let (w, h, d) = (self.ancho as i32, self.alto as i32, self.fondo as i32);
        for _ in 0..150 {
            let x = 4 + rnd.next_int(w - 8);
            let y = 4 + rnd.next_int(h - 8);
            let Some(z) = self.altura_superficie(x, y) else { continue };
            if self.get(x, y, z) != tiles::CESPED || z <= SEA_LEVEL {
                continue;
            }
            let alto = 4 + rnd.next_int(3);
            if z + alto + 2 >= d {
                continue;
            }
            for i in 1..=alto {
                let zi = z + i;
                if self.get(x, y, zi) == tiles::AIRE || self.get(x, y, zi) == tiles::HOJAS {
                    let j = self.idx(x as usize, y as usize, zi as usize);
                    self.blocks[j] = tiles::TRONCO;
                }
            }
            let cima = z + alto;
            let (dz_min, dz_max) = match copa {
                Copa::Roble => (-1, 2),
                _ => (-1, 1),
            };
            for dz in dz_min..=dz_max {
                let r: i32 = match (copa, dz) {
                    (Copa::Roble, 2) => 1,
                    (Copa::Roble, 1) => 1,
                    (Copa::Roble, _) => 2,
                    (_, 1) => 1,
                    _ => 2,
                };
                for dx in -r..=r {
                    for dy in -r..=r {
                        match copa {
                            Copa::Cuadrada => {
                                if dz != 1 && dx.abs() == r && dy.abs() == r {
                                    continue;
                                }
                            }
                            Copa::Cruz => {
                                if dz == 1 {
                                    if dx != 0 && dy != 0 {
                                        continue;
                                    }
                                } else if dx.abs() == r && dy.abs() == r {
                                    continue;
                                }
                            }
                            Copa::Roble => {
                                if dz == 2 && dx != 0 && dy != 0 {
                                    continue;
                                }
                            }
                        }
                        let (lx, ly, lz) = (x + dx, y + dy, cima + dz);
                        if lx < 0 || ly < 0 || lz < 0 || lx >= w || ly >= h || lz >= d {
                            continue;
                        }
                        if self.get(lx, ly, lz) == tiles::AIRE {
                            let j = self.idx(lx as usize, ly as usize, lz as usize);
                            self.blocks[j] = tiles::HOJAS;
                        }
                    }
                }
            }
        }
    }

    // Lagos de lava de superficie. El primer 0.0.12a tenia lagos, no solo
    // bloques sueltos; estos son pequenos y se colocan sobre tierra firme.
    fn anadir_lagos_lava(&mut self, seed: i64, superficie: &[i32]) {
        let mut rnd = perlin::JavaRandom::new(seed ^ 0x1A2A_910B);
        let (w, h) = (self.ancho as i32, self.alto as i32);
        let fondo = self.fondo as i32;
        for _ in 0..16 {
            let cx = 4 + rnd.next_int(w - 8);
            let cy = 4 + rnd.next_int(h - 8);
            let k = cx as usize + cy as usize * self.ancho;
            let alto = superficie[k].clamp(SEA_LEVEL + 2, fondo - 3);
            for dx in -2..=2 {
                for dy in -2..=2 {
                    let x = cx + dx;
                    let y = cy + dy;
                    if x < 1 || y < 1 || x >= w - 1 || y >= h - 1 {
                        continue;
                    }
                    let k = x as usize + y as usize * self.ancho;
                    let z = superficie[k].clamp(alto, fondo - 2);
                    self.set(x, y, z, tiles::LAVA);
                    if z + 1 < fondo {
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
        let (w, h) = (self.ancho as i32, self.alto as i32);
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

    /// `LevelGen.generateLevel` de c0.0.13a-dev. La build que se conserva hace
    /// un mundo plano: en la cota `depth/2` hay hierba, debajo tierra y en el
    /// tercio inferior piedra. Después cava los túneles y siembra lava en las
    /// cuevas; el agua no llega a aparecer porque todas las columnas están
    /// cerradas hasta la superficie.
    fn generar_013a_dev(&mut self, seed: i64) {
        let (w, h, d) = (self.ancho, self.alto, self.fondo);
        for x in 0..w {
            for y in 0..d {
                for z in 0..h {
                    let id = if y == d / 2 && y >= d / 2 - 1 {
                        tiles::CESPED
                    } else if y <= d / 2 {
                        tiles::TIERRA
                    } else {
                        tiles::AIRE
                    };
                    let id = if y <= d / 3 { tiles::PIEDRA } else { id };
                    let i = self.idx(x, z, y);
                    self.blocks[i] = id;
                }
            }
        }
        self.cavar_cuevas(seed);
        self.anadir_lava_013a_dev(seed);
    }

    /// `LevelGen.addLava` de 0.0.13a-dev. El original lanza
    /// width*height*depth/10000 fuentes en la mitad inferior del mundo y las
    /// rellena con flood fill. Aquí se colocan en bloques de aire de las cuevas.
    fn anadir_lava_013a_dev(&mut self, seed: i64) {
        let (w, h, d) = (self.ancho, self.alto, self.fondo);
        let mut rnd = perlin::JavaRandom::new(seed ^ 0x013A_0001);
        let count = w * h * d / 10000;
        for _ in 0..count {
            let x = rnd.next_int(w as i32);
            let y = rnd.next_int((d / 2) as i32).max(1);
            let z = rnd.next_int(h as i32);
            if self.get(x, z, y) == tiles::AIRE {
                self.flood_lava(x, z, y);
            }
        }
    }

    /// Relleno de lava de `LevelGen.floodFillLiquid`: una balsa conectada en
    /// el mismo plano vertical. El original usa un flood fill de dos ejes a la
    /// altura de la fuente; aquí se recorre con una pila y se limita a las
    /// celdas de aire, para no atravesar roca.
    fn flood_lava(&mut self, x: i32, z: i32, y: i32) {
        let mut pila = vec![(x, z)];
        while let Some((px, pz)) = pila.pop() {
            if px < 0 || pz < 0 || px >= self.ancho as i32 || pz >= self.alto as i32 {
                continue;
            }
            if self.get(px, pz, y) != tiles::AIRE {
                continue;
            }
            self.set(px, pz, y, tiles::LAVA);
            pila.push((px + 1, pz));
            pila.push((px - 1, pz));
            pila.push((px, pz + 1));
            pila.push((px, pz - 1));
        }
    }

    // LevelGen.generateMap de c0.0.11a. Identico al de la familia B salvo los
    // cuatro NoiseMap y las cuevas (LevelGen se las hace aqui, no en Level).
    fn generar_clasico(&mut self, seed: i64) {
        let (w, h, d) = (self.ancho, self.alto, self.fondo);
        let sem = semillas(seed);
        let h1 = noisemap::read(0, w, h, sem[0]);
        let h2 = noisemap::read(0, w, h, sem[1]);
        let cf = noisemap::read(1, w, h, sem[2]);
        let rock = noisemap::read(1, w, h, sem[3]);
        let d = d as i32;
        for x in 0..w {
            for y in 0..h {
                for z in 0..d as usize {
                    let k = x + y * w;
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
    // roca. Los túneles salen de un Random que en el original se crea con la
    // hora; aqui va con semilla para que el mundo se reproduzca.
    fn cavar_cuevas(&mut self, seed: i64) {
        let mut random = perlin::JavaRandom::new(seed ^ 0xC0FFEE);
        let (w, h, d) = (self.ancho as i32, self.alto as i32, self.fondo as i32);
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
        (z * self.alto + y) * self.ancho + x
    }

    // generateMap() del original.
    fn generar(&mut self, seed: i64) {
        let (w, h, d) = (self.ancho, self.alto, self.fondo);
        let sem = semillas(seed);
        let h1 = perlin::read(0, w, h, sem[0]);
        let h2 = perlin::read(0, w, h, sem[1]);
        let cf = perlin::read(1, w, h, sem[2]);
        let rock = perlin::read(1, w, h, sem[3]);
        let d = d as i32;
        for x in 0..w {
            for y in 0..h {
                for z in 0..d as usize {
                    let k = x + y * w;
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
        for x in x0..(x0 + x1).min(self.ancho) {
            for y in y0..(y0 + y1).min(self.alto) {
                let mut z = self.fondo - 1;
                while z > 0 && !self.is_light_blocker(x, z, y) {
                    z -= 1;
                }
                self.light[x + y * self.ancho] = z as i32;
            }
        }
    }

    pub fn get(&self, x: i32, y: i32, z: i32) -> u8 {
        if x < 0
            || y < 0
            || z < 0
            || x as usize >= self.ancho
            || y as usize >= self.alto
            || z as usize >= self.fondo
        {
            return 0;
        }
        self.blocks[self.idx(x as usize, y as usize, z as usize)]
    }

    pub fn set(&mut self, x: i32, y: i32, z: i32, id: u8) -> bool {
        if x < 0
            || y < 0
            || z < 0
            || x as usize >= self.ancho
            || y as usize >= self.alto
            || z as usize >= self.fondo
        {
            return false;
        }
        let i = self.idx(x as usize, y as usize, z as usize);
        if self.blocks[i] == id {
            return false;
        }
        let antes = self.blocks[i];
        self.blocks[i] = id;
        self.dirty.push((x, y));
        if tiles::es_liquido(id) && self.liquidos.len() < 8192 {
            self.liquidos.push((x, y, z));
        }
        if tiles::cae(id) {
            self.asentar(x, y, z);
        } else if id == tiles::AIRE && antes != tiles::AIRE {
            self.asentar_columna(x, y, z + 1);
        }
        self.calc_light(x as usize, y as usize, 1, 1);
        true
    }

    /// Arena y grava se asientan de golpe: mientras debajo haya aire o un
    /// liquido, el bloque baja. No usa `set` para no encadenar asentamientos.
    fn asentar(&mut self, x: i32, y: i32, z: i32) {
        let id = self.get(x, y, z);
        if !tiles::cae(id) {
            return;
        }
        let mut zz = z;
        while zz > 0 {
            let abajo = self.get(x, y, zz - 1);
            if abajo == tiles::AIRE || tiles::es_liquido(abajo) {
                let i = self.idx(x as usize, y as usize, zz as usize);
                let j = self.idx(x as usize, y as usize, (zz - 1) as usize);
                self.blocks[i] = tiles::AIRE;
                self.blocks[j] = id;
                self.dirty.push((x, y));
                zz -= 1;
            } else {
                break;
            }
        }
        if zz != z {
            self.calc_light(x as usize, y as usize, 1, 1);
        }
    }

    /// Si se quita el soporte, la arena/grava que tenga encima cae tambien.
    fn asentar_columna(&mut self, x: i32, y: i32, z0: i32) {
        for z in z0..self.fondo as i32 {
            if tiles::cae(self.get(x, y, z)) {
                self.asentar(x, y, z);
            }
        }
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
            || x as usize >= self.ancho
            || y as usize >= self.alto
            || z as usize >= self.fondo
        {
            return true;
        }
        z >= self.light[x as usize + y as usize * self.ancho]
    }

    // Cajas de colisión de los bloques que tocan la zona dada (Level.getCubes).
    pub fn cubes(&self, aabb: &BlockBox) -> Vec<BlockBox> {
        let mut out = Vec::new();
        let x0 = (aabb.x0.floor() as i32).max(0);
        let x1 = ((aabb.x1 + 1.0).floor() as i32).min(self.ancho as i32);
        let y0 = (aabb.y0.floor() as i32).max(0);
        let y1 = ((aabb.y1 + 1.0).floor() as i32).min(self.alto as i32);
        let z0 = (aabb.z0.floor() as i32).max(0);
        let z1 = ((aabb.z1 + 1.0).floor() as i32).min(self.fondo as i32);
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

    fn dimensiones(&self) -> (usize, usize, usize) {
        (self.ancho, self.alto, self.fondo)
    }
}

// ------------------------------------------------ tick de los tiles (Level.tick)

impl Nivel {
    // Level.tick: acumula todos los bloques del mundo y procesa un tile al azar
    // por cada 400 acumulados (con 256x256x64 son ~10.000 por tick).
    pub fn tick(&mut self) {
        self.ticks = self.ticks.wrapping_add(1);
        self.unprocessed += (self.ancho * self.alto * self.fondo) as i32;
        let mut ticks = self.unprocessed / 400;
        self.unprocessed -= ticks * 400;
        while ticks > 0 {
            let x = self.random.next_int(self.ancho as i32);
            let z = self.random.next_int(self.fondo as i32);
            let y = self.random.next_int(self.alto as i32);
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
        let mut diferidos = Vec::new();
        while i < self.liquidos.len() && hechos < max {
            let (x, y, z) = self.liquidos[i];
            i += 1;
            let id = self.get(x, y, z);
            if !tiles::es_liquido(id) {
                continue;
            }
            // 0.0.15a: la lava se expande a la mitad de velocidad. Se guarda
            // para el tick siguiente en vez de perder el frente de flujo.
            if id == tiles::LAVA && self.lava_lenta && self.ticks % 2 == 0 {
                diferidos.push((x, y, z));
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
        self.liquidos.extend(diferidos);
        // Si el jugador vacia un oceano entero, no dejamos una cola sin fin.
        if self.liquidos.len() > 8192 {
            self.liquidos.truncate(8192);
        }
    }
}

// ------------------------------------------------------------ guardar/cargar
// Hasta 0.0.12a_03: GZIP del array de bloques tal cual. Desde 0.0.13a:
// GZIP( magic 0x271BB788, version 1, writeUTF(nombre), writeUTF(creador),
// writeLong(fecha), writeShort(width), writeShort(height), writeShort(depth),
// bloques ). El array sigue el orden (y * height + z) * width + x.

fn leer_utf(datos: &[u8], pos: &mut usize) -> Option<String> {
    if *pos + 2 > datos.len() {
        return None;
    }
    let len = u16::from_be_bytes([datos[*pos], datos[*pos + 1]]) as usize;
    *pos += 2;
    if *pos + len > datos.len() {
        return None;
    }
    let s = String::from_utf8(datos[*pos..*pos + len].to_vec()).ok()?;
    *pos += len;
    Some(s)
}

fn escribir_utf(out: &mut Vec<u8>, s: &str) {
    let bytes = s.as_bytes();
    let len = u16::try_from(bytes.len()).expect("nombre de mundo demasiado largo");
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(bytes);
}

// El jar usa ids separados para el agua/lava en reposo (9/11) y para sus
// versiones animadas (8/10). El motor del port solo tiene un agua y una lava,
// asi que al leer un level.dat de Java se normalizan y al escribir se vuelven
// a codificar como las variantes en reposo.
fn id_desde_java(id: u8) -> u8 {
    match id {
        9 => tiles::AGUA,
        10 | 11 => tiles::LAVA,
        12 => tiles::ARENA,
        13 => tiles::GRAVA,
        14 => tiles::MENA_ORO,
        15 => tiles::MENA_HIERRO,
        16 => tiles::MENA_CARBON,
        17 => tiles::TRONCO,
        18 => tiles::HOJAS,
        _ => id,
    }
}

fn id_a_java(id: u8) -> u8 {
    match id {
        tiles::AGUA => 9,
        tiles::LAVA => 11,
        tiles::ARENA => 12,
        tiles::GRAVA => 13,
        tiles::MENA_ORO => 14,
        tiles::MENA_HIERRO => 15,
        tiles::MENA_CARBON => 16,
        tiles::TRONCO => 17,
        tiles::HOJAS => 18,
        _ => id,
    }
}

/// Decodifica la cabecera de 0.0.13a. Devuelve None si el fichero no la tiene
/// o si el array de bloques no mide width*height*depth.
fn decodificar_cabecera(
    datos: &[u8],
) -> Option<(usize, usize, usize, Vec<u8>, String, String, i64)> {
    if datos.len() < 4 + 1 + 2 + 2 + 8 + 6 {
        return None;
    }
    let mut pos = 0usize;
    let magic = i32::from_be_bytes(datos[pos..pos + 4].try_into().ok()?);
    pos += 4;
    if magic != MAGIC_LEVEL {
        return None;
    }
    if datos[pos] != VERSION_LEVEL {
        return None;
    }
    pos += 1;
    let nombre = leer_utf(datos, &mut pos)?;
    let creador = leer_utf(datos, &mut pos)?;
    let creado = i64::from_be_bytes(datos[pos..pos + 8].try_into().ok()?);
    pos += 8;
    let ancho = u16::from_be_bytes([datos[pos], datos[pos + 1]]) as usize;
    pos += 2;
    let alto = u16::from_be_bytes([datos[pos], datos[pos + 1]]) as usize;
    pos += 2;
    let fondo = u16::from_be_bytes([datos[pos], datos[pos + 1]]) as usize;
    pos += 2;
    let largo = ancho.checked_mul(alto)?.checked_mul(fondo)?;
    if datos.len() - pos != largo {
        return None;
    }
    let mut blocks = datos[pos..].to_vec();
    for b in &mut blocks {
        *b = id_desde_java(*b);
    }
    Some((ancho, alto, fondo, blocks, nombre, creador, creado))
}

impl Nivel {
    fn codificar_cabecera(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(4 + 1 + 2 + 2 + 8 + 6 + self.blocks.len());
        out.extend_from_slice(&MAGIC_LEVEL.to_be_bytes());
        out.push(VERSION_LEVEL);
        escribir_utf(&mut out, &self.nombre);
        escribir_utf(&mut out, &self.creador);
        out.extend_from_slice(&self.creado.to_be_bytes());
        out.extend_from_slice(&(self.ancho as u16).to_be_bytes());
        out.extend_from_slice(&(self.alto as u16).to_be_bytes());
        out.extend_from_slice(&(self.fondo as u16).to_be_bytes());
        out.extend(self.blocks.iter().map(|b| id_a_java(*b)));
        out
    }

    fn save_at(&self, path: &Path) -> io::Result<()> {
        let datos = match self.formato {
            Formato::Crudo => self.blocks.clone(),
            Formato::Cabecera13a => self.codificar_cabecera(),
        };
        crate::world::save_gzip_atomic(path, &datos)
    }

    // GZIP normal, como el DataOutputStream(GZIPOutputStream) del original.
    pub fn save(&self) -> io::Result<()> {
        self.save_at(Path::new("level.dat"))
    }

    fn load_at(&mut self, path: &Path) -> Option<()> {
        let datos = crate::world::load_gzip_any(path)?;
        if let Some((ancho, alto, fondo, blocks, nombre, creador, creado)) =
            decodificar_cabecera(&datos)
        {
            self.ancho = ancho;
            self.alto = alto;
            self.fondo = fondo;
            self.blocks = blocks;
            self.light = vec![0; ancho * alto];
            self.nombre = nombre;
            self.creador = creador;
            self.creado = creado;
            self.formato = Formato::Cabecera13a;
            return Some(());
        }
        // Formato antiguo: solo bloques, sin dimensiones. Solo vale para el
        // tamaño con el que se construyó este Nivel.
        if datos.len() == self.ancho * self.alto * self.fondo {
            self.blocks.copy_from_slice(&datos);
            return Some(());
        }
        None
    }

    // Lee cualquier level.dat gzip (también los del Java original): primero
    // prueba la cabecera de 0.0.13a y, si no, el formato crudo antiguo.
    pub fn load(&mut self) -> Option<()> {
        self.load_at(Path::new("level.dat"))
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
        let mut n = Nivel::nuevo_vacio(seed, WIDTH, HEIGHT, DEPTH);
        n.generar_clasico_012(seed);

        n
    }

    #[test]
    fn el_agua_colocada_se_extiende() {
        let mut n = Nivel::nuevo_vacio(1, WIDTH, HEIGHT, DEPTH);
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
    fn la_020_siembra_flores_y_la_esponja_absorbe_agua() {
        // Flores sobre una pradera plana de hierba.
        let mut n = Nivel::nuevo_vacio(7, 64, 64, 32);
        for x in 0..64 {
            for y in 0..64 {
                n.set(x, y, 10, tiles::CESPED);
            }
        }
        n.sembrar_plantas(7);
        let flores = n
            .blocks
            .iter()
            .filter(|id| matches!(**id, tiles::DENTELEON | tiles::ROSA))
            .count();
        assert!(flores > 0, "la 0.0.20a no sembro ninguna flor");

        // Esponja: un cubo de agua de 5x5x5 desaparece con ella.
        let mut n = Nivel::nuevo_vacio(1, 16, 16, 16);
        for x in 5..10 {
            for y in 5..10 {
                for z in 5..10 {
                    n.set(x, y, z, tiles::AGUA);
                }
            }
        }
        n.set(7, 7, 7, tiles::ESPONJA);
        let absorbidos = n.absorber_esponja(7, 7, 7);
        assert!(absorbidos > 0);
        assert_eq!(n.get(7, 7, 7), tiles::AIRE);
        assert_eq!(n.get(5, 5, 5), tiles::AIRE);
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
        for x in 0..n.ancho {
            for y in 0..n.alto {
                assert_eq!(n.get(x as i32, y as i32, 0), tiles::BEDROCK);
                for z in 0..n.fondo {
                    match n.get(x as i32, y as i32, z as i32) {
                        tiles::AGUA => {
                            agua += 1;
                            if agua_pos.is_none()
                                && z + 1 < n.fondo
                                && n.get(x as i32, y as i32, z as i32 + 1) == tiles::AIRE
                            {
                                agua_pos = Some((x, y, z));
                            }
                            if agua_playa.len() < 8
                                && z + 1 < n.fondo
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
                                && z + 1 < n.fondo
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

    #[test]
    fn el_013a03_no_genera_lava_sobre_el_mar() {
        let mut n = Nivel::nuevo_vacio(12345, WIDTH, HEIGHT, DEPTH);
        n.generar_clasico_012_con(12345, false);
        let mut lava_bajo = 0usize;
        for x in 0..n.ancho {
            for y in 0..n.alto {
                for z in 0..n.fondo {
                    if n.get(x as i32, y as i32, z as i32) == tiles::LAVA {
                        assert!((z as i32) < SEA_LEVEL, "lava sobre el mar en {x},{y},{z}");
                        lava_bajo += 1;
                    }
                }
            }
        }
        assert!(lava_bajo > 0, "no queda lava en las cuevas: {lava_bajo}");
    }

    #[test]
    fn el_013a_dev_es_plano_largo_y_sin_lava_superficial() {
        let n = Nivel::new_013a_dev(12345);
        assert_eq!((n.ancho, n.alto, n.fondo), (32, 512, 64));
        let mut lava = 0;
        for x in 0..n.ancho {
            for y in 0..n.alto {
                assert_eq!(n.get(x as i32, y as i32, 32), tiles::CESPED);
                assert_eq!(n.get(x as i32, y as i32, 31), tiles::TIERRA);
                assert_eq!(n.get(x as i32, y as i32, 22), tiles::TIERRA);
                for z in 0..n.fondo {
                    let id = n.get(x as i32, y as i32, z as i32);
                    if z > 32 {
                        assert_eq!(id, tiles::AIRE, "bloque sobre el plano en {x},{y},{z}");
                    }
                    if id == tiles::LAVA {
                        lava += 1;
                    }
                }
            }
        }
        assert!(lava > 0, "la build documenta lava en las cuevas");
    }

    #[test]
    fn la_cabecera_013a_guarda_dimensiones_y_bloques() {
        let dir = std::env::temp_dir().join(format!(
            "rustium-cabecera-{}-{}",
            std::process::id(),
            ahora_millis()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("level.dat");
        let mut n = Nivel::nuevo_vacio(7, 32, 512, 64);
        n.formato = Formato::Cabecera13a;
        n.nombre = "A Nice World".to_string();
        n.creador = "noname".to_string();
        n.blocks[0] = tiles::PIEDRA;
        let ultimo = n.blocks.len() - 1;
        n.blocks[ultimo] = tiles::CESPED;
        n.save_at(&path).unwrap();

        let mut leido = Nivel::nuevo_vacio(0, 1, 1, 1);
        leido.load_at(&path).unwrap();
        assert_eq!((leido.ancho, leido.alto, leido.fondo), (32, 512, 64));
        assert_eq!(leido.formato, Formato::Cabecera13a);
        assert_eq!(leido.nombre, "A Nice World");
        assert_eq!(leido.creador, "noname");
        assert_eq!(leido.get(0, 0, 0), tiles::PIEDRA);
        assert_eq!(leido.blocks, n.blocks);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn la_cabecera_normaliza_los_ids_de_liquidos_del_jar() {
        let mut n = Nivel::nuevo_vacio(0, 2, 2, 2);
        n.formato = Formato::Cabecera13a;
        let agua = n.blocks.len() - 2;
        let lava = n.blocks.len() - 1;
        n.blocks[agua] = tiles::AGUA;
        n.blocks[lava] = tiles::LAVA;
        let datos = n.codificar_cabecera();
        let (_, _, _, bloques, _, _, _) = decodificar_cabecera(&datos).unwrap();
        assert_eq!(bloques[agua], tiles::AGUA);
        assert_eq!(bloques[lava], tiles::LAVA);
        // En el archivo viajan como calmWater/calmLava del jar.
        let inicio = datos.len() - n.blocks.len();
        assert_eq!(datos[inicio + agua], 9);
        assert_eq!(datos[inicio + lava], 11);
    }

    #[test]
    fn la_014_siembra_playas_menas_y_arboles() {
        // Generar sin pasar por load(): en el directorio del proyecto puede
        // haber un level.dat de pruebas que taparia el mundo nuevo.
        let mut n = Nivel::vacio(12_345, WIDTH, HEIGHT, DEPTH, Formato::Cabecera13a);
        n.generar_clasico_014_con(12_345, Copa::Cuadrada, false);
        let mut arena = 0;
        let mut menas = 0;
        let mut troncos = 0;
        let mut hojas = 0;
        for &b in &n.blocks {
            match b {
                tiles::ARENA => arena += 1,
                tiles::MENA_ORO | tiles::MENA_HIERRO | tiles::MENA_CARBON => menas += 1,
                tiles::TRONCO => troncos += 1,
                tiles::HOJAS => hojas += 1,
                _ => {}
            }
        }
        assert!(arena > 0, "la 0.0.14a tiene playas de arena");
        assert!(menas > 0, "la 0.0.14a tiene menas");
        assert!(troncos > 0, "la 0.0.14a tiene arboles");
        assert!(hojas > troncos, "cada tronco lleva su copa de hojas");
    }

    #[test]
    fn la_arena_y_la_grava_caen_hasta_el_fondo() {
        let mut n = Nivel::nuevo_vacio(0, 4, 4, 8);
        n.set(1, 1, 0, tiles::PIEDRA);
        n.set(1, 1, 6, tiles::ARENA);
        assert_eq!(n.get(1, 1, 1), tiles::ARENA);
        assert_eq!(n.get(1, 1, 6), tiles::AIRE);
        // La grava tambien, y al quitar el soporte cae lo que tenga encima.
        n.set(2, 2, 0, tiles::PIEDRA);
        n.set(2, 2, 1, tiles::GRAVA);
        n.set(2, 2, 2, tiles::GRAVA);
        assert_eq!(n.get(2, 2, 1), tiles::GRAVA);
        n.set(2, 2, 0, tiles::AIRE);
        assert_eq!(n.get(2, 2, 0), tiles::GRAVA);
    }

    #[test]
    fn la_copa_de_los_arboles_de_014a_es_cuadrada() {
        let mut n = Nivel::vacio(12_345, WIDTH, HEIGHT, DEPTH, Formato::Cabecera13a);
        n.generar_clasico_014_con(12_345, Copa::Cuadrada, false);
        let mut copas = 0;
        for x in 1..(n.ancho - 1) {
            for y in 1..(n.alto - 1) {
                for z in 1..(n.fondo - 1) {
                    if n.get(x as i32, y as i32, z as i32) != tiles::HOJAS {
                        continue;
                    }
                    if n.get(x as i32, y as i32, z as i32 + 1) != tiles::AIRE {
                        continue;
                    }
                    let mut completa = true;
                    for dx in -1..=1 {
                        for dy in -1..=1 {
                            if dx == 0 && dy == 0 {
                                continue;
                            }
                            if n.get((x as i32) + dx, (y as i32) + dy, z as i32) != tiles::HOJAS {
                                completa = false;
                            }
                        }
                    }
                    if completa {
                        copas += 1;
                    }
                }
            }
        }
        assert!(copas > 0, "la copa superior debe ser un 3x3 completo");
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

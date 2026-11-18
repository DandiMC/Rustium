// Tiles de la familia rd-160052 / rd-20090515: cada bloque es un id con su
// textura por cara (el cesped usa una distinta arriba, abajo y en los lados),
// y sus propiedades de solidez y de bloqueo de luz.
//
// UV: el original calcula u0 = (tex % 16) / 16 y v0 = (tex / 16) / 16, o sea que
// el atlas es de 16x16 tiles de 16 px (256x256), y suma 0.0624375 (un tile).

pub const AIRE: u8 = 0;
pub const PIEDRA: u8 = 1;
pub const CESPED: u8 = 2;
pub const TIERRA: u8 = 3;
pub const LADRILLO: u8 = 4;
pub const MADERA: u8 = 5;
pub const ARBUSTO: u8 = 6;

/// Textura (indice de tile del atlas) de una cara concreta del bloque.
/// Caras como en el original: 0 = abajo, 1 = arriba, 2 = -y, 3 = +y, 4 = -x, 5 = +x.
/// (En nuestro motor: 0 = abajo, 1 = arriba, 2/3 = los dos lados del eje y,
/// 4/5 = los dos lados del eje x.)
pub fn textura(id: u8, cara: usize) -> u32 {
    match id {
        PIEDRA => 1,
        CESPED => match cara {
            1 => 0, // arriba: hierba
            0 => 2, // abajo: tierra
            _ => 3, // lados: hierba lateral
        },
        TIERRA => 2,
        LADRILLO => 16,
        MADERA => 4,
        ARBUSTO => 15,
        _ => 0,
    }
}

pub fn solido(id: u8) -> bool {
    match id {
        ARBUSTO => false,
        _ => id != AIRE,
    }
}

pub fn bloquea_luz(id: u8) -> bool {
    match id {
        ARBUSTO => false,
        _ => id != AIRE,
    }
}

/// Factor de luz por cara, igual que Tile.render del original:
/// arriba/abajo 1.0, los dos lados del eje z 0.8 y los del eje x 0.6.
pub fn factor_cara(cara: usize) -> f32 {
    match cara {
        0 | 1 => 1.0,
        2 | 3 => 0.8,
        _ => 0.6,
    }
}

// Tile.tick de la familia B: solo el cesped y el arbusto lo implementan.
// `random` es el del nivel (java.util.Random) y el orden de las llamadas importa.
pub fn tick(nivel: &mut crate::legacy::level::Nivel, id: u8, x: i32, y: i32, z: i32) {
    match id {
        CESPED => {
            if !nivel.is_lit(x, y, z) {
                nivel.set(x, y, z, TIERRA);
            } else {
                for _ in 0..4 {
                    let rx = nivel.random.next_int(3) - 1;
                    let rz = nivel.random.next_int(5) - 3; // vertical
                    let ry = nivel.random.next_int(3) - 1; // profundidad
                    let (xt, zt, yt) = (x + rx, z + rz, y + ry);
                    if nivel.get(xt, yt, zt) == TIERRA && nivel.is_lit(xt, yt, zt) {
                        nivel.set(xt, yt, zt, CESPED);
                    }
                }
            }
        }
        ARBUSTO => {
            let abajo = nivel.get(x, y, z - 1);
            if !nivel.is_lit(x, y, z) || (abajo != TIERRA && abajo != CESPED) {
                nivel.set(x, y, z, AIRE);
            }
        }
        _ => {}
    }
}

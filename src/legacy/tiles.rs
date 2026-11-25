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
// c0.0.12a: la capa que cierra el mundo, el agua y la lava. Los ids 1-6 ya
// existian en c0.0.11a, asi que un mundo viejo se sigue leyendo igual.
pub const BEDROCK: u8 = 7;
pub const AGUA: u8 = 8;
pub const LAVA: u8 = 9;

pub fn es_liquido(id: u8) -> bool {
    id == AGUA || id == LAVA
}

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
        BEDROCK => 7,
        AGUA => 8,
        LAVA => 9,
        _ => 0,
    }
}

/// Textura base del tile (`Tile.tex`), la que usa `Tile.destroy` para las
/// particulas. El cesped suelta su lateral (3), no la cara de abajo (2).
pub fn textura_particula(id: u8) -> u32 {
    match id {
        CESPED => 3,
        _ => textura(id, 1),
    }
}

pub fn solido(id: u8) -> bool {
    // Tile.isSolid() es true por defecto, pero Bush lo sobrescribe a false y su
    // getAABB() devuelve null: se atraviesa y no frena al jugador.
    id != AIRE && id != ARBUSTO && !es_liquido(id)
}

pub fn bloquea_luz(id: u8) -> bool {
    // Bush.blocksLight() tambien devuelve false: deja pasar la luz.
    id != AIRE && id != ARBUSTO && !es_liquido(id)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_cesped_suelta_particulas_con_su_textura_lateral() {
        assert_eq!(textura_particula(CESPED), 3);
        assert_eq!(textura_particula(PIEDRA), 1);
        assert_eq!(textura_particula(TIERRA), 2);
    }

    #[test]
    fn el_arbusto_se_atraviesa_y_deja_pasar_la_luz() {
        // Bush sobrescribe isSolid() y blocksLight() a false; su getAABB() es
        // null, asi que tampoco genera cajas de colision.
        assert!(!solido(ARBUSTO));
        assert!(!bloquea_luz(ARBUSTO));
        assert!(!solido(AIRE));
        assert!(!bloquea_luz(AIRE));
        // El agua y la lava de 0.0.12a se atraviesan y dejan pasar la luz.
        assert!(!solido(AGUA));
        assert!(!bloquea_luz(LAVA));
        // Los bloques normales siguen siendo solidos y opacos.
        assert!(solido(PIEDRA));
        assert!(bloquea_luz(PIEDRA));
        assert!(solido(BEDROCK));
    }
}

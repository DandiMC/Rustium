// Malla de un chunk para la familia B.
//
// A diferencia de rd-132211, aqui la luz no se muestrea: cada cara lleva solo su
// factor fijo (1.0 arriba/abajo, 0.8 profundidad, 0.6 lateral) y se dibuja si el
// vecino no es solido, repartida en las dos pasadas de `Tile.render`:
// capa 0 = caras que dan a aire iluminado, capa 1 = caras que dan a aire a
// oscuras (bajo un techo o dentro de una cueva). El original dibuja la capa 1
// con niebla exponencial densa y casi negra.
//
// Las esquinas y las UV son las de `Tile.renderFace`, cara por cara, en el mismo
// orden (importa: es lo que hace que la hierba salga con el fleco arriba).

use super::level::{Nivel, DEPTH, HEIGHT, WIDTH};
use super::tiles;
use crate::render::Vertex;

pub const CHUNK: usize = 16;

// (vecino dx, dy=profundidad, dz=vertical, cara, esquinas [(x, z, y, u, v)])
const CARAS: [(i32, i32, i32, usize, [[f32; 5]; 4]); 6] = [
    // cara 0: abajo (y-1 en el Java)
    (0, 0, -1, 0, [
        [0.0, 0.0, 1.0, 0.0, 1.0],
        [0.0, 0.0, 0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0, 1.0, 0.0],
        [1.0, 0.0, 1.0, 1.0, 1.0],
    ]),
    // cara 1: arriba
    (0, 0, 1, 1, [
        [1.0, 1.0, 1.0, 1.0, 1.0],
        [1.0, 1.0, 0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 1.0, 0.0, 1.0],
    ]),
    // cara 2: fondo (z-1 en el Java = nuestro y-1)
    (0, -1, 0, 2, [
        [0.0, 1.0, 0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0, 0.0, 1.0],
        [0.0, 0.0, 0.0, 1.0, 1.0],
    ]),
    // cara 3: frente
    (0, 1, 0, 3, [
        [0.0, 1.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0, 1.0],
        [1.0, 0.0, 1.0, 1.0, 1.0],
        [1.0, 1.0, 1.0, 1.0, 0.0],
    ]),
    // cara 4: x-1
    (-1, 0, 0, 4, [
        [0.0, 1.0, 1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0, 1.0, 1.0],
    ]),
    // cara 5: x+1
    (1, 0, 0, 5, [
        [1.0, 0.0, 1.0, 0.0, 1.0],
        [1.0, 0.0, 0.0, 1.0, 1.0],
        [1.0, 1.0, 0.0, 1.0, 0.0],
        [1.0, 1.0, 1.0, 0.0, 0.0],
    ]),
];

/// Los dos planos cruzados del arbusto, tal cual los emite `Bush.render`: giros
/// de 45 y 135 grados, centro del bloque, de suelo a techo, textura 15 y blanco
/// (sin sombra). Devuelve 8 vertices: 4 por plano; el original los emite dos
/// veces, uno por sentido, porque hay back-face culling.
///
/// Ojo con los ejes: `Bush.render` usa (x, altura, fondo), pero aqui `x` e `y`
/// son horizontales (y = fondo) y `z` es la altura. El vertice se guarda como
/// [x, z, y], igual que las caras de `construir`.
pub fn arbusto(x: f32, y: f32, z: f32) -> [([f32; 3], [f32; 2]); 8] {
    let tex = 15u32;
    let u0 = (tex % 16) as f32 / 16.0;
    let v0 = (tex / 16) as f32 / 16.0;
    let duv = 0.0624375f32;
    let mut out = [([0.0f32; 3], [0.0f32; 2]); 8];
    for r in 0..2usize {
        let a = r as f32 * std::f32::consts::PI / 2.0 + std::f32::consts::FRAC_PI_4;
        let xa = a.sin() * 0.5;
        let ya = a.cos() * 0.5;
        let (x0, x1) = (x + 0.5 - xa, x + 0.5 + xa);
        let (y0, y1) = (y + 0.5 - ya, y + 0.5 + ya);
        let (z0, z1) = (z, z + 1.0);
        let quad = [
            ([x0, z1, y0], [u0 + duv, v0]),
            ([x1, z1, y1], [u0, v0]),
            ([x1, z0, y1], [u0, v0 + duv]),
            ([x0, z0, y0], [u0 + duv, v0 + duv]),
        ];
        for (i, v) in quad.iter().enumerate() {
            out[r * 4 + i] = *v;
        }
    }
    out
}

pub fn construir(nivel: &Nivel, cx: usize, cy: usize, capa: u8) -> (Vec<Vertex>, Vec<u32>) {
    let iluminada = capa == 0;
    let mut verts = Vec::new();
    let mut idx: Vec<u32> = Vec::new();
    let x0 = cx * CHUNK;
    let y0 = cy * CHUNK;
    for x in x0..(x0 + CHUNK).min(WIDTH) {
        for y in y0..(y0 + CHUNK).min(HEIGHT) {
            for z in 0..DEPTH {
                let id = nivel.get(x as i32, y as i32, z as i32);
                if id == tiles::AIRE {
                    continue;
                }
                if id == tiles::ARBUSTO {
                    // Bush.render elige capa con su propio isLit; sus dos planos van
                    // y vuelven (culling) y la textura tiene alfa que el shader corta.
                    if nivel.is_lit(x as i32, y as i32, z as i32) == iluminada {
                        let base = verts.len() as u32;
                        for (pos, uv) in arbusto(x as f32, y as f32, z as f32) {
                            verts.push(Vertex { pos, uv, shade: 1.0 });
                        }
                        for q in 0..2u32 {
                            let b = base + q * 4;
                            idx.extend_from_slice(&[
                                b, b + 1, b + 2, b, b + 2, b + 3,
                                b + 2, b + 1, b, b + 3, b + 2, b,
                            ]);
                        }
                    }
                    continue;
                }
                for (dx, dy, dz, cara, esquinas) in CARAS.iter() {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    let nz = z as i32 + dz;
                    if nivel.is_solid(nx, ny, nz) {
                        continue;
                    }
                    // Dos bloques del mismo liquido no dibujan la cara que los
                    // separa; si no, el oceano seria una pila de paredes azules.
                    if tiles::es_liquido(id) && nivel.get(nx, ny, nz) == id {
                        continue;
                    }
                    // shouldRenderFace del original: `!isSolid && isLit ^ layer == 1`.
                    if nivel.is_lit(nx, ny, nz) != iluminada {
                        continue;
                    }
                    let tex = tiles::textura(id, *cara);
                    let u0 = (tex % 16) as f32 / 16.0;
                    let v0 = (tex / 16) as f32 / 16.0;
                    let duv = 0.0624375f32;
                    let factor = tiles::factor_cara(*cara);
                    let base = verts.len() as u32;
                    for e in esquinas.iter() {
                        verts.push(Vertex {
                            pos: [x as f32 + e[0], z as f32 + e[1], y as f32 + e[2]],
                            uv: [u0 + e[3] * duv, v0 + e[4] * duv],
                            shade: factor,
                        });
                    }
                    idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
                }
            }
        }
    }
    (verts, idx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_capa_1_dibuja_las_caras_que_dan_a_aire_a_oscuras() {
        let mut n = Nivel {
            blocks: vec![0; WIDTH * HEIGHT * DEPTH],
            light: vec![0; WIDTH * HEIGHT],
            random: crate::legacy::perlin::JavaRandom::new(0),
            unprocessed: 0,
            dirty: Vec::new(),
            liquidos: Vec::new(),
        };
        // Piedra en (0,0,0) con la columna a oscuras desde z=1: la cara de
        // arriba da a aire sin luz (solo capa 1); las otras cinco dan a aire
        // iluminado (capa 0, incluidos los bordes del mapa).
        n.blocks[0] = tiles::PIEDRA;
        n.light[0] = 2;
        let (_, i0) = construir(&n, 0, 0, 0);
        let (_, i1) = construir(&n, 0, 0, 1);
        assert_eq!(i0.len(), 5 * 6);
        assert_eq!(i1.len(), 6);
    }

    #[test]
    fn el_arbusto_son_dos_planos_cruzados() {
        let v = arbusto(10.0, 20.0, 30.0);
        let d = std::f32::consts::FRAC_1_SQRT_2 * 0.5;
        // Primer plano a 45 grados: vertical de z=30 a z=31, diagonal por el
        // centro del bloque en x=10.5 y fondo=20.5.
        assert!((v[0].0[0] - (10.5 - d)).abs() < 1e-5);
        assert!((v[0].0[1] - 31.0).abs() < 1e-5);
        assert!((v[0].0[2] - (20.5 - d)).abs() < 1e-5);
        assert!((v[2].0[1] - 30.0).abs() < 1e-5);
        // UV: tile 15, u de 0.9375 a 0.9999375, v de 0 a 0.0624375
        assert!((v[0].1[0] - 0.9999375).abs() < 1e-6);
        assert!((v[0].1[1] - 0.0).abs() < 1e-6);
        assert!((v[2].1[1] - 0.0624375).abs() < 1e-6);
        // Segundo plano, a 135 grados: el mismo cuadrado cruzado
        assert!((v[4].0[2] - (20.5 + d)).abs() < 1e-5);
        assert!((v[6].0[2] - (20.5 - d)).abs() < 1e-5);
    }
}

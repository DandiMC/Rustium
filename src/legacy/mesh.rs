// Malla de un chunk para la familia B.
//
// A diferencia de rd-132211, aqui la luz no se muestrea: cada cara lleva solo su
// factor fijo (1.0 arriba/abajo, 0.8 profundidad, 0.6 lateral) y se dibuja si el
// vecino no es solido y esta iluminado: `!isSolid && isLit` del original
// (pase opaco, layer == 0).
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

pub fn construir(nivel: &Nivel, cx: usize, cy: usize) -> (Vec<Vertex>, Vec<u32>) {
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
                for (dx, dy, dz, cara, esquinas) in CARAS.iter() {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    let nz = z as i32 + dz;
                    if nivel.is_solid(nx, ny, nz) || !nivel.is_lit(nx, ny, nz) {
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

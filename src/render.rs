// Modulo de renderizado (wgpu).

use crate::world::{Level, AIR};
use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::window::Window;

pub const CHUNK: usize = 16;

pub const SHADER_WGSL: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    fog_color: vec4<f32>,
    camera_pos: vec4<f32>,
    fog_density: f32,
    // 1.0 = GL_EXP (rd-132211); 0.0 = la cuadratica de la familia nueva
    fog_exp: f32,
};
@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var tex: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;

struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) shade: f32,
    // Distancia en el eje de la vista (|z_eye|): es la que usa glFog, no la
    // distancia euclidea.
    @location(2) fogz: f32,
};

@vertex
fn vs_main(@location(0) pos: vec3<f32>, @location(1) uv: vec2<f32>, @location(2) shade: f32) -> VOut {
    var o: VOut;
    let clip = u.view_proj * vec4<f32>(pos, 1.0);
    o.clip = clip;
    o.uv = uv;
    o.shade = shade;
    o.fogz = clip.w;
    return o;
}

@fragment
fn fs_main(v: VOut) -> @location(0) vec4<f32> {
    let c = textureSample(tex, samp, v.uv);
    // GL_ALPHA_TEST con GL_GREATER 0.5: el original descarta el texel, no lo
    // mezcla. Es lo que recorta las hojas del arbusto (tile 15 del atlas).
    if (c.a < 0.5) {
        discard;
    }
    let dist = max(v.fogz, 0.0);
    var fog = 1.0 - exp(-dist * dist * u.fog_density * u.fog_density);
    if (u.fog_exp > 0.5) {
        fog = 1.0 - exp(-dist * u.fog_density);
    }
    let rgb = mix(c.rgb * v.shade, u.fog_color.rgb, clamp(fog, 0.0, 1.0));
    return vec4<f32>(rgb, 1.0);
}
"#;

// Resaltado del bloque apuntado. Modo romper: una sola cara, la que se esta
// mirando, con una capa blanca translucida (no se enciende el bloque entero).
// Modo construir: el bloque elegido dibujado con terrain.png en la casilla donde
// caeria, con blending normal y escribiendo profundidad para que se vea macizo.
// El shader sirve para los dos: el velo usa un texel blanco de 1x1 y el preview
// el atlas, con el color uniforme modulando brillo y alfa.
pub const HIT_SHADER: &str = r#"
struct HitU { view_proj: mat4x4<f32>, model: mat4x4<f32>, color: vec4<f32>, };
@group(0) @binding(0) var<uniform> u: HitU;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;
struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) shade: f32,
};
@vertex fn vs(@location(0) p: vec3<f32>, @location(1) uv: vec2<f32>, @location(2) s: f32) -> VOut {
    var o: VOut;
    o.clip = u.view_proj * u.model * vec4<f32>(p, 1.0);
    o.uv = uv;
    o.shade = s;
    return o;
}
@fragment fn fs(v: VOut) -> @location(0) vec4<f32> {
    let c = textureSample(tex, samp, v.uv);
    // El atlas corta las hojas con alfa; el texel blanco del velo siempre pasa.
    if (c.a < 0.5) { discard; }
    return vec4<f32>(c.rgb * v.shade * u.color.rgb, u.color.a);
}
"#;

// Modelos de personaje (zombis): piel char.png y el mismo esquema de niebla que
// el terreno (la capa 0 va sin niebla; la capa 1 usa el uniform oscuro). El
// original tambien mete a los zombis sin luz en la pasada de niebla.
pub const MODEL_SHADER: &str = r#"
struct MU {
    view_proj: mat4x4<f32>,
    fog_color: vec4<f32>,
    camera_pos: vec4<f32>,
    fog_density: f32,
    fog_exp: f32,
    _pad: vec2<f32>,
};
@group(0) @binding(0) var<uniform> u: MU;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;
struct VIn { @location(0) p: vec3<f32>, @location(1) uv: vec2<f32> };
struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) fogz: f32,
};
@vertex fn vs(v: VIn) -> VOut {
    var o: VOut;
    o.clip = u.view_proj * vec4<f32>(v.p, 1.0);
    o.uv = v.uv;
    o.fogz = o.clip.w;
    return o;
}
@fragment fn fs(v: VOut) -> @location(0) vec4<f32> {
    let c = textureSample(tex, samp, v.uv);
    let dist = max(v.fogz, 0.0);
    var fog = 1.0 - exp(-dist * dist * u.fog_density * u.fog_density);
    if (u.fog_exp > 0.5) {
        fog = 1.0 - exp(-dist * u.fog_density);
    }
    let rgb = mix(c.rgb, u.fog_color.rgb, clamp(fog, 0.0, 1.0));
    return vec4<f32>(rgb, 1.0);
}
"#;

// Caras en el orden de hit_cube_vertices (0 arriba, 1 abajo, 2 +x, 3 -x,
// 4 +z, 5 -z) llevadas a la numeracion de Tile.textura, y su brillo fijo como
// en la malla del chunk: arriba/abajo 1.0, lados z 0.8, lados x 0.6.
const HIT_CARA_TILE: [usize; 6] = [1, 0, 5, 4, 3, 2];
const HIT_CARA_BRILLO: [f32; 6] = [1.0, 1.0, 0.6, 0.6, 0.8, 0.8];

// UV de las 4 esquinas de cada cara, en el mismo orden que sus vertices y con
// el mismo reparto que Tile.renderFace (el de legacy::mesh::CARAS).
const HIT_CARA_UV: [[[f32; 2]; 4]; 6] = [
    [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]],
    [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
    [[1.0, 1.0], [1.0, 0.0], [0.0, 0.0], [0.0, 1.0]],
    [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
    [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
    [[1.0, 1.0], [1.0, 0.0], [0.0, 0.0], [0.0, 1.0]],
];

// renderHit con mode 1 dibuja `Tile.tiles[tileType].render` en la casilla
// contigua: el bloque elegido con su textura, sus brillos por cara y blending
// normal. `bloque` elige el preview; para la seleccion de una sola cara esta
// hit_face_vertices.
fn hit_cube_vertices((bx, by, bz): (i32, i32, i32), bloque: Option<u8>) -> [Vertex; 36] {
    let esquinas: [[[f32; 3]; 4]; 6] = [
        [[0.0, 1.0, 0.0], [0.0, 1.0, 1.0], [1.0, 1.0, 1.0], [1.0, 1.0, 0.0]],
        [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 1.0], [0.0, 0.0, 1.0]],
        // Mismo sentido que las otras cinco caras (las traseras se descartan).
        [[1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [1.0, 1.0, 1.0], [1.0, 0.0, 1.0]],
        [[0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 1.0], [0.0, 1.0, 0.0]],
        [[0.0, 0.0, 1.0], [1.0, 0.0, 1.0], [1.0, 1.0, 1.0], [0.0, 1.0, 1.0]],
        [[0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0, 0.0], [1.0, 0.0, 0.0]],
    ];
    let duv = 0.0624375f32;
    let mut out = [Vertex { pos: [0.0; 3], uv: [0.0, 0.0], shade: 1.0 }; 36];
    for (fi, c) in esquinas.iter().enumerate() {
        let (u0, v0, brillo, uvs) = match bloque {
            Some(id) => {
                let tex = hud_textura(id, HIT_CARA_TILE[fi]);
                (
                    (tex % 16) as f32 / 16.0,
                    (tex / 16) as f32 / 16.0,
                    HIT_CARA_BRILLO[fi],
                    HIT_CARA_UV[fi],
                )
            }
            // Velo blanco: da igual la UV (el bind group es un texel blanco).
            None => (0.0, 0.0, 1.0, [[0.0, 0.0]; 4]),
        };
        for (vi, p) in c.iter().enumerate() {
            out[fi * 6 + vi] = Vertex {
                pos: [bx as f32 + p[0], bz as f32 + p[1], by as f32 + p[2]],
                uv: [u0 + uvs[vi][0] * duv, v0 + uvs[vi][1] * duv],
                shade: brillo,
            };
        }
        out[fi * 6 + 4] = out[fi * 6];
        out[fi * 6 + 5] = out[fi * 6 + 2];
    }
    out
}

// El arbusto no es un cubo: Bush.render son dos planos cruzados. El preview de
// colocacion llama al mismo render, asi que tiene que salir la planta y no una
// cara de bloque con hojas. Se emiten los dos sentidos de cada plano porque el
// pipeline descarta las caras traseras.
fn hit_bush_vertices((bx, by, bz): (i32, i32, i32)) -> Vec<Vertex> {
    let tex = 15u32;
    let u0 = (tex % 16) as f32 / 16.0;
    let v0 = (tex / 16) as f32 / 16.0;
    let duv = 0.0624375f32;
    let mut out = Vec::with_capacity(24);
    for r in 0..2usize {
        let a = r as f32 * std::f32::consts::PI / 2.0 + std::f32::consts::FRAC_PI_4;
        let xa = a.sin() * 0.5;
        let za = a.cos() * 0.5;
        let (x0, x1) = (bx as f32 + 0.5 - xa, bx as f32 + 0.5 + xa);
        let (z0, z1) = (bz as f32 + 0.5 - za, bz as f32 + 0.5 + za);
        let (y0, y1) = (by as f32, by as f32 + 1.0);
        let quad = [
            ([x0, y1, z0], [u0 + duv, v0]),
            ([x1, y1, z1], [u0, v0]),
            ([x1, y0, z1], [u0, v0 + duv]),
            ([x0, y0, z0], [u0 + duv, v0 + duv]),
        ];
        for e in [0usize, 1, 2, 0, 2, 3, 0, 2, 1, 0, 3, 2] {
            out.push(Vertex { pos: quad[e].0, uv: quad[e].1, shade: 1.0 });
        }
    }
    out
}

fn hit_vertices((bx, by, bz): (i32, i32, i32), bloque: Option<u8>) -> Vec<Vertex> {
    match bloque {
        Some(6) => hit_bush_vertices((bx, by, bz)),
        None => hit_cube_vertices((bx, by, bz), None).to_vec(),
        Some(id) => hit_cube_vertices((bx, by, bz), Some(id)).to_vec(),
    }
}

// Vertices (2 triangulos) de la cara indicada de un bloque, con la textura y el
// brillo que le tocarian en la malla del chunk. Misma tabla de caras que
// build_chunk_mesh: si se toca una hay que tocar la otra.
fn hit_face_vertices((bx, by, bz): (i32, i32, i32), n: (i32, i32, i32)) -> [Vertex; 6] {
    let faces: [(i32, i32, i32, f32, (i32, i32, i32), [[f32; 3]; 4]); 6] = [
        (0, 0, 1, 1.0, (0, 0, 0), [[0.0, 1.0, 0.0], [0.0, 1.0, 1.0], [1.0, 1.0, 1.0], [1.0, 1.0, 0.0]]),
        (0, 0, -1, 1.0, (0, 0, -1), [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 1.0], [0.0, 0.0, 1.0]]),
        (1, 0, 0, 0.6, (1, 0, 0), [[1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [1.0, 1.0, 1.0], [1.0, 0.0, 1.0]]),
        (-1, 0, 0, 0.6, (-1, 0, 0), [[0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 1.0], [0.0, 1.0, 0.0]]),
        (0, 1, 0, 0.8, (0, 1, 0), [[0.0, 0.0, 1.0], [1.0, 0.0, 1.0], [1.0, 1.0, 1.0], [0.0, 1.0, 1.0]]),
        (0, -1, 0, 0.8, (0, -1, 0), [[0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0, 0.0], [1.0, 0.0, 0.0]]),
    ];
    let fi = match n {
        (0, 0, 1) => 0,
        (0, 0, -1) => 1,
        (1, 0, 0) => 2,
        (-1, 0, 0) => 3,
        (0, 1, 0) => 4,
        (0, -1, 0) => 5,
        // Normal cero: el ojo esta dentro del bloque. El original dibuja la inferior.
        _ => 1,
    };
    let (_, _, _, _, _, corners) = faces[fi];
    // Un unico texel para toda la cara y sin brillo, como el original: renderFace
    // solo emite vertices, asi que el velo lleva la coordenada que quedo puesta
    // (0,0) en los 4 y el color del glColor4f. Antes se texturizaba la cara y se
    // elegia el tile por altura, y en un lateral del borde superior salia la
    // textura de arriba: parecia una capa blanca puesta encima del bloque.
    let shade = 1.0;
    let uv = [0.0, 0.0];
    // La cara va unos micrones por delante del bloque. Compartir plano con la
    // cara del chunk hacia que ambos fragmentos compitieran por el mismo valor de
    // profundidad (z-fighting); separarlos 1 mm es imperceptible a simple vista y
    // deja la capa siempre delante, sin tocar el depth bias de la GPU.
    let eps = 0.001f32;
    let off = [
        n.0 as f32 * eps,
        n.2 as f32 * eps,
        n.1 as f32 * eps,
    ];
    let mut p = [Vertex { pos: [0.0; 3], uv, shade }; 4];
    for (ci, c) in corners.iter().enumerate() {
        p[ci] = Vertex {
            pos: [
                bx as f32 + c[0] + off[0],
                bz as f32 + c[1] + off[1],
                by as f32 + c[2] + off[2],
            ],
            uv,
            shade,
        };
    }
    [p[0], p[1], p[2], p[0], p[2], p[3]]
}

// Overlay de texto (F3). Los vertices llegan ya en coordenadas de recorte, asi que
// el vertex shader no transforma nada y el fragment shader solo devuelve el color.
// Texto del HUD con la fuente del jar: muestrea default.gif y tinta con el
// color uniforme, como Tesselator con GL_MODULATE y el glColor de Font.
pub const TEXT_SHADER: &str = r#"
struct TU { color: vec4<f32>, };
@group(0) @binding(0) var<uniform> u: TU;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;
struct VOut { @builtin(position) clip: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) shade: f32 };
@vertex fn vs(@location(0) p: vec3<f32>, @location(1) uv: vec2<f32>, @location(2) s: f32) -> VOut {
    var o: VOut;
    o.clip = vec4<f32>(p, 1.0);
    o.uv = uv;
    o.shade = s;
    return o;
}
@fragment fn fs(v: VOut) -> @location(0) vec4<f32> {
    let c = textureSample(tex, samp, v.uv);
    if (c.a < 0.5) { discard; }
    return vec4<f32>(u.color.rgb * v.shade * c.rgb, u.color.a * c.a);
}
"#;

pub const UI_SHADER: &str = r#"
struct UiU { color: vec4<f32>, };
@group(0) @binding(0) var<uniform> u: UiU;
@vertex fn vs(@location(0) p: vec3<f32>) -> @builtin(position) vec4<f32> {
    return vec4<f32>(p, 1.0);
}
@fragment fn fs() -> @location(0) vec4<f32> { return u.color; }
"#;

// Overlay con color por vertice: la tarjeta de "captura realizada" necesita
// varios colores en el mismo draw (panel, borde, insignia y textos), y el
// pipeline de UI usa un unico color uniforme.
pub const UI_COLOR_SHADER: &str = r#"
struct VOut { @builtin(position) clip: vec4<f32>, @location(0) color: vec4<f32> };
@vertex fn vs(@location(0) p: vec3<f32>, @location(1) c: vec4<f32>) -> VOut {
    var o: VOut;
    o.clip = vec4<f32>(p, 1.0);
    o.color = c;
    return o;
}
@fragment fn fs(v: VOut) -> @location(0) vec4<f32> { return v.color; }
"#;

// Vertices reservados para el overlay (cada pixel del glifo son 6 vertices).
const UI_MAX_VERTS: usize = 32768;

// Fuente 5x7 minima: solo lo que necesita el contador.
const FONT_CHARS: &[(char, [u8; 7])] = &[
    ('0', [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110]),
    ('1', [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110]),
    ('2', [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111]),
    ('3', [0b11111, 0b00010, 0b00100, 0b00010, 0b00001, 0b10001, 0b01110]),
    ('4', [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010]),
    ('5', [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110]),
    ('6', [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110]),
    ('7', [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000]),
    ('8', [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110]),
    ('9', [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100]),
    ('.', [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00110, 0b00110]),
    (',', [0b00000, 0b00000, 0b00000, 0b00000, 0b00110, 0b00110, 0b00100]),
    (':', [0b00000, 0b00110, 0b00110, 0b00000, 0b00110, 0b00110, 0b00000]),
    ('-', [0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000]),
    ('/', [0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000]),
    ('a', [0b00000, 0b00000, 0b01110, 0b00001, 0b01111, 0b10001, 0b01111]),
    ('b', [0b10000, 0b10000, 0b10110, 0b11001, 0b10001, 0b10001, 0b11110]),
    ('c', [0b00000, 0b00000, 0b01110, 0b10001, 0b10000, 0b10001, 0b01110]),
    ('d', [0b00001, 0b00001, 0b01101, 0b10011, 0b10001, 0b10001, 0b01111]),
    ('e', [0b00000, 0b00000, 0b01110, 0b10001, 0b11111, 0b10000, 0b01110]),
    ('f', [0b00110, 0b01001, 0b01000, 0b11100, 0b01000, 0b01000, 0b01000]),
    ('g', [0b00000, 0b01111, 0b10001, 0b10001, 0b01111, 0b00001, 0b01110]),
    ('h', [0b10000, 0b10000, 0b10110, 0b11001, 0b10001, 0b10001, 0b10001]),
    ('i', [0b00100, 0b00000, 0b01100, 0b00100, 0b00100, 0b00100, 0b01110]),
    ('j', [0b00010, 0b00000, 0b00110, 0b00010, 0b00010, 0b10010, 0b01100]),
    ('k', [0b10000, 0b10000, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010]),
    ('l', [0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110]),
    ('m', [0b00000, 0b00000, 0b11010, 0b10101, 0b10101, 0b10001, 0b10001]),
    ('n', [0b00000, 0b00000, 0b10110, 0b11001, 0b10001, 0b10001, 0b10001]),
    ('o', [0b00000, 0b00000, 0b01110, 0b10001, 0b10001, 0b10001, 0b01110]),
    ('p', [0b00000, 0b00000, 0b11110, 0b10001, 0b11110, 0b10000, 0b10000]),
    ('q', [0b00000, 0b00000, 0b01101, 0b10011, 0b01111, 0b00001, 0b00001]),
    ('r', [0b00000, 0b00000, 0b10110, 0b11001, 0b10000, 0b10000, 0b10000]),
    ('s', [0b00000, 0b00000, 0b01111, 0b10000, 0b01110, 0b00001, 0b11110]),
    ('t', [0b01000, 0b01000, 0b11100, 0b01000, 0b01000, 0b01001, 0b00110]),
    ('u', [0b00000, 0b00000, 0b10001, 0b10001, 0b10001, 0b10011, 0b01101]),
    ('v', [0b00000, 0b00000, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100]),
    ('w', [0b00000, 0b00000, 0b10001, 0b10001, 0b10101, 0b10101, 0b01010]),
    ('x', [0b00000, 0b00000, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001]),
    ('y', [0b00000, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110]),
    ('z', [0b00000, 0b00000, 0b11111, 0b00010, 0b00100, 0b01000, 0b11111]),
    ('F', [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000]),
    ('P', [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000]),
    ('S', [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110]),
    ('A', [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001]),
    ('B', [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110]),
    ('C', [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110]),
    ('D', [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110]),
    ('E', [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111]),
    ('G', [0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111]),
    ('H', [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001]),
    ('I', [0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110]),
    ('J', [0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100]),
    ('K', [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001]),
    ('L', [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111]),
    ('M', [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001]),
    ('N', [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001]),
    ('O', [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110]),
    ('Q', [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101]),
    ('R', [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001]),
    ('T', [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100]),
    ('U', [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110]),
    ('V', [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100]),
    ('W', [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010]),
    ('X', [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001]),
    ('Y', [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100]),
    ('Z', [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111]),
    ('+', [0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000]),
];

// Genera los triangulos del texto en coordenadas de recorte. Cada pixel encendido
// de la fuente es un quad de lado `p` pixeles de PANTALLA (usando el ancho y el alto
// reales para que el glifo no salga estirado por el aspecto de la ventana).
fn ui_text_quads(text: &str, width: u32, height: u32) -> Vec<f32> {
    let p = 3.0f32;
    let margin = 8.0f32;
    let dx = 2.0 / width.max(1) as f32;
    let dy = 2.0 / height.max(1) as f32;
    let mut out: Vec<f32> = Vec::new();
    let mut cx = margin;
    let mut cy = 0.0f32;
    for ch in text.chars() {
        // Segunda linea (coordenadas debajo del FPS) sin tocar la fuente.
        if ch == '\n' {
            cx = margin;
            // El glifo mide 7 px a escala 3: 26 px deja aire entre lineas.
            cy += 26.0;
            continue;
        }
        if let Some((_, rows)) = FONT_CHARS.iter().find(|(c, _)| *c == ch) {
            for (r, bits) in rows.iter().enumerate() {
                for c in 0..5u32 {
                    if *bits & (1u8 << (4 - c)) != 0 {
                        let x0 = -1.0 + (cx + c as f32 * p) * dx;
                        let y0 = 1.0 - (margin + cy + r as f32 * p) * dy;
                        let x1 = x0 + p * dx;
                        let y1 = y0 - p * dy;
                        out.extend_from_slice(&[x0, y0, 0.0, x1, y0, 0.0, x1, y1, 0.0]);
                        out.extend_from_slice(&[x0, y0, 0.0, x1, y1, 0.0, x0, y1, 0.0]);
                    }
                }
            }
        }
        cx += 6.0 * p;
    }
    out
}

// Texto con la fuente real del jar (default.gif) para el HUD de Classic.
// Font.java pinta el glifo entero de 8x8 y avanza por charWidths, con una copia
// oscura desplazada 1 px (drawShadow). Aqui los vertices llevan la UV del glifo
// y el brillo: 1.0 el texto y 0.247 la sombra ((0xFFFFFF & 0xFCFCFC) >> 2).
const FUENTE_SOMBRA: f32 = 0.247;

fn fuente_anchos(rgba: &[u8], w: usize, h: usize) -> [u8; 128] {
    let mut anchos = [0u8; 128];
    for i in 0..128usize {
        let xt = i % 16;
        let yt = i / 16;
        let mut x = 0;
        let mut columna_vacia = false;
        while x < 8 && !columna_vacia {
            columna_vacia = true;
            let mut y = 0;
            while y < 8 {
                let px = xt * 8 + x;
                let py = yt * 8 + y;
                if px < w && py < h && rgba[(py * w + px) * 4 + 2] > 128 {
                    columna_vacia = false;
                }
                y += 1;
            }
            x += 1;
        }
        if i == 32 {
            x = 4;
        }
        anchos[i] = x as u8;
    }
    anchos
}

fn fuente_quads(
    texto: &str,
    x_gui: f32,
    y_gui: f32,
    ancho: u32,
    alto: u32,
    fw: f32,
    fh: f32,
    anchos: &[u8; 128],
    sombra: bool,
    out: &mut Vec<Vertex>,
) {
    let sh = 240.0f32;
    let sw = ancho as f32 * sh / alto.max(1) as f32;
    let mut cx = x_gui + if sombra { 1.0 } else { 0.0 };
    let cy = y_gui + if sombra { 1.0 } else { 0.0 };
    let shade = if sombra { FUENTE_SOMBRA } else { 1.0 };
    for ch in texto.chars() {
        let c = ch as usize;
        if c < 128 {
            let (u0, v0) = ((c % 16) as f32 * 8.0 / fw, (c / 16) as f32 * 8.0 / fh);
            let (u1, v1) = (u0 + 8.0 / fw, v0 + 8.0 / fh);
            let (x0, x1) = (cx, cx + 8.0);
            let (y0, y1) = (cy, cy + 8.0);
            let p = |px: f32, py: f32| [2.0 * px / sw - 1.0, 1.0 - 2.0 * py / sh, 0.5];
            let (a, b, d, e) = (p(x0, y0), p(x1, y0), p(x1, y1), p(x0, y1));
            let v = |pos: [f32; 3], uv: [f32; 2]| Vertex { pos, uv, shade };
            out.extend_from_slice(&[
                v(a, [u0, v0]), v(b, [u1, v0]), v(d, [u1, v1]),
                v(a, [u0, v0]), v(d, [u1, v1]), v(e, [u0, v1]),
            ]);
        }
        cx += anchos[c.min(127)] as f32;
    }
}

// Texto del HUD en coordenadas de la GUI del original (alto 240). Cada pixel
// de la fuente es una unidad de GUI, asi que a 854x480 la letra mide 8 px y a
// 1024x768 unos 25, como el default.gif del jar.
fn ui_rect(out: &mut Vec<f32>, x: f32, y: f32, w: f32, h: f32, color: [f32; 4], width: u32, height: u32) {
    let dx = 2.0 / width.max(1) as f32;
    let dy = 2.0 / height.max(1) as f32;
    let x0 = -1.0 + x * dx;
    let x1 = -1.0 + (x + w) * dx;
    let y0 = 1.0 - y * dy;
    let y1 = 1.0 - (y + h) * dy;
    let v = |px: f32, py: f32| [px, py, 0.0, color[0], color[1], color[2], color[3]];
    let (a, b, d, e) = (v(x0, y0), v(x1, y0), v(x1, y1), v(x0, y1));
    out.extend_from_slice(&a);
    out.extend_from_slice(&b);
    out.extend_from_slice(&d);
    out.extend_from_slice(&a);
    out.extend_from_slice(&d);
    out.extend_from_slice(&e);
}

fn ui_texto_color(out: &mut Vec<f32>, text: &str, width: u32, height: u32, x: f32, y: f32, escala: f32, color: [f32; 4]) {
    let dx = 2.0 / width.max(1) as f32;
    let dy = 2.0 / height.max(1) as f32;
    let mut cx = x;
    for ch in text.chars() {
        if let Some((_, rows)) = FONT_CHARS.iter().find(|(c, _)| *c == ch) {
            for (r, bits) in rows.iter().enumerate() {
                for c in 0..5u32 {
                    if *bits & (1u8 << (4 - c)) != 0 {
                        let px = cx + c as f32 * escala;
                        let py = y + r as f32 * escala;
                        ui_rect(out, px, py, escala, escala, color, width, height);
                    }
                }
            }
        }
        cx += 6.0 * escala;
    }
}

// Tarjeta de captura: entra deslizandose desde la derecha, la insignia late,
// la barra se llena y se va sola. Se dibuja en el renderer compartido, asi que
// sale en las 14 versiones.
const TOAST_DURACION: f32 = 2.6;
const TOAST_ANCHO: f32 = 292.0;
const TOAST_ALTO: f32 = 58.0;

fn toast_quads(edad: f32, width: u32, height: u32, y_base: f32) -> Vec<f32> {
    let mut out: Vec<f32> = Vec::new();
    if !(0.0..TOAST_DURACION).contains(&edad) || width < 360 {
        return out;
    }
    let entrada = (edad / 0.30).min(1.0);
    let ease = 1.0 - (1.0 - entrada).powi(3);
    let salida = ((edad - (TOAST_DURACION - 0.40)) / 0.40).clamp(0.0, 1.0);
    let ease_sal = salida * salida;
    let panel_w = TOAST_ANCHO.min(width as f32 - 28.0);
    let panel_h = TOAST_ALTO;
    // La posicion se mide en pixeles de pantalla, como el resto del HUD.
    let x = width as f32 - panel_w - 14.0
        + (1.0 - ease) * (panel_w + 24.0)
        + ease_sal * (panel_w + 24.0);
    let y = y_base - (1.0 - ease) * 5.0;
    let a = (1.0 - salida).clamp(0.0, 1.0);
    let violeta = [0.63, 0.44, 1.0, 0.95 * a];
    let oro = [1.0, 0.78, 0.20, a];

    // Sombra y panel con borde violeta y filo dorado a la izquierda.
    ui_rect(&mut out, x + 4.0, y + 4.0, panel_w, panel_h, [0.0, 0.0, 0.0, 0.30 * a], width, height);
    ui_rect(&mut out, x, y, panel_w, panel_h, [0.075, 0.045, 0.13, 0.94 * a], width, height);
    ui_rect(&mut out, x, y, panel_w, 2.0, violeta, width, height);
    ui_rect(&mut out, x, y + panel_h - 2.0, panel_w, 2.0, violeta, width, height);
    ui_rect(&mut out, x, y, 2.0, panel_h, violeta, width, height);
    ui_rect(&mut out, x + panel_w - 2.0, y, 2.0, panel_h, violeta, width, height);
    ui_rect(&mut out, x, y, 5.0, panel_h, oro, width, height);
    // Esquinas doradas, el detalle de tarjeta de recompensa.
    ui_rect(&mut out, x + panel_w - 22.0, y, 22.0, 3.0, oro, width, height);
    ui_rect(&mut out, x + panel_w - 3.0, y + panel_h - 22.0, 3.0, 22.0, oro, width, height);

    // Brillo que barre la tarjeta al entrar.
    let barrido = ((edad - 0.14) / 0.52).clamp(0.0, 1.0);
    if barrido > 0.0 && barrido < 1.0 {
        let sx = x - 26.0 + barrido * (panel_w + 52.0);
        ui_rect(&mut out, sx, y + 2.0, 26.0, panel_h - 4.0, [1.0, 1.0, 1.0, 0.055 * a], width, height);
        ui_rect(&mut out, sx + 26.0, y + 2.0, 8.0, panel_h - 4.0, [1.0, 1.0, 1.0, 0.035 * a], width, height);
    }

    // Insignia: marco dorado con una camara de pixeles.
    let ix = x + 10.0;
    let iy = y + 9.0;
    ui_rect(&mut out, ix, iy, 40.0, 40.0, [0.13, 0.08, 0.24, a], width, height);
    ui_rect(&mut out, ix, iy, 40.0, 2.0, oro, width, height);
    ui_rect(&mut out, ix, iy + 38.0, 40.0, 2.0, oro, width, height);
    ui_rect(&mut out, ix, iy, 2.0, 40.0, oro, width, height);
    ui_rect(&mut out, ix + 38.0, iy, 2.0, 40.0, oro, width, height);
    // Cuerpo, visor y flash de la camara.
    ui_rect(&mut out, ix + 8.0, iy + 16.0, 24.0, 15.0, oro, width, height);
    ui_rect(&mut out, ix + 14.0, iy + 12.0, 10.0, 4.0, oro, width, height);
    ui_rect(&mut out, ix + 16.0, iy + 19.0, 9.0, 9.0, [0.13, 0.08, 0.24, a], width, height);
    ui_rect(&mut out, ix + 18.0, iy + 21.0, 5.0, 5.0, [1.0, 0.92, 0.55, a], width, height);
    ui_rect(&mut out, ix + 27.0, iy + 13.0, 4.0, 3.0, [0.13, 0.08, 0.24, a], width, height);

    // Textos: titulo blanco y recompensa en dorado.
    ui_texto_color(&mut out, "CAPTURA REALIZADA", width, height, x + 60.0, y + 9.0, 2.0, [1.0, 1.0, 1.0, a]);
    ui_texto_color(&mut out, "+1 CAPTURA", width, height, x + 60.0, y + 28.0, 2.0, oro);

    // Barra de XP: fondo, relleno que sube y punta que brilla al completarse.
    let bx = x + 60.0;
    let by = y + 47.0;
    let bw = panel_w - 72.0;
    ui_rect(&mut out, bx, by, bw, 5.0, [0.18, 0.12, 0.28, a], width, height);
    let progreso = ((edad - 0.10) / 0.55).clamp(0.0, 1.0);
    let lleno = 1.0 - (1.0 - progreso).powi(2);
    let pulso = 0.80 + 0.20 * ((edad * 10.0).sin() * 0.5 + 0.5);
    let brillo = [1.0, 0.84, 0.30, pulso * a];
    ui_rect(&mut out, bx, by, bw * lleno, 5.0, brillo, width, height);
    if progreso > 0.02 {
        ui_rect(&mut out, bx + (bw * lleno - 5.0).max(0.0), by - 1.0, 5.0, 7.0, [1.0, 1.0, 0.85, a], width, height);
    }
    out
}

fn ui_text_gui(text: &str, width: u32, height: u32, x_gui: f32, y_gui: f32) -> Vec<f32> {
    let sh = 240.0f32;
    let sw = width as f32 * sh / height.max(1) as f32;
    let mut out: Vec<f32> = Vec::new();
    let mut cx = x_gui;
    for ch in text.chars() {
        if let Some((_, rows)) = FONT_CHARS.iter().find(|(c, _)| *c == ch) {
            for (r, bits) in rows.iter().enumerate() {
                for c in 0..5u32 {
                    if *bits & (1u8 << (4 - c)) != 0 {
                        let x0 = 2.0 * (cx + c as f32) / sw - 1.0;
                        let x1 = 2.0 * (cx + c as f32 + 1.0) / sw - 1.0;
                        let y0 = 1.0 - 2.0 * (y_gui + r as f32) / sh;
                        let y1 = 1.0 - 2.0 * (y_gui + r as f32 + 1.0) / sh;
                        out.extend_from_slice(&[x0, y0, 0.0, x1, y0, 0.0, x1, y1, 0.0]);
                        out.extend_from_slice(&[x0, y0, 0.0, x1, y1, 0.0, x0, y1, 0.0]);
                    }
                }
            }
        }
        cx += 6.0;
    }
    out
}

// Cruz de mira del original (drawGui): en su ortográfica la pantalla mide 240
// de alto, y la cruz son dos barras de 9x1 px centradas. Aqui se pasan a clip
// space con el mismo criterio (y hacia abajo).
fn cruz_quads(width: u32, height: u32) -> Vec<f32> {
    let sh = 240.0f32;
    let sw = width as f32 * 240.0 / height.max(1) as f32;
    let cx = (sw / 2.0).round();
    let cy = sh / 2.0;
    let mut out: Vec<f32> = Vec::new();
    let mut quad = |x0: f32, y0: f32, x1: f32, y1: f32| {
        let c = |px: f32, py: f32| [2.0 * px / sw - 1.0, 1.0 - 2.0 * py / sh];
        let (a, b, d, e) = (c(x0, y0), c(x1, y0), c(x1, y1), c(x0, y1));
        out.extend_from_slice(&[a[0], a[1], 0.0, b[0], b[1], 0.0, d[0], d[1], 0.0]);
        out.extend_from_slice(&[a[0], a[1], 0.0, d[0], d[1], 0.0, e[0], e[1], 0.0]);
    };
    // Barra vertical (1 px de ancho, de hc-4 a hc+5)
    quad(cx, cy - 4.0, cx + 1.0, cy + 5.0);
    // Barra horizontal (de wc-4 a wc+5, 1 px de alto)
    quad(cx - 4.0, cy, cx + 5.0, cy + 1.0);
    out
}

// Cubo del bloque elegido, como en RubyDung.drawGui: en la ortografica de 240 de
// alto (esquina superior derecha), girado 30 y 45 grados y espejado en x e y.
// Cada cara se emite con los dos sentidos de vertices porque el espejo cambia la
// orientacion y el pipeline de terreno descarta las caras traseras; con la
// profundidad por delante, el sentido que no toca queda tapado.
const HUD_VERTS: usize = 6 * 12;

fn hud_proyecta(p: [f32; 3], sw: f32, sh: f32) -> [f32; 3] {
    // Misma cadena que el original: scale(-1,-1,1), translate(-1.5,0.5,-0.5),
    // rotacion 45 en Y, rotacion 30 en X, scale(16) y translate(sw-16, 16).
    let mut x = -p[0];
    let mut y = -p[1];
    let mut z = p[2];
    x -= 1.5;
    y += 0.5;
    z -= 0.5;
    // Giro en Y del original (45): al ir despues del espejo de x, el sentido
    // util es el contrario; comprobado contra el reparto de luces por cara del
    // drawGui (0.8 a la izquierda, 0.6 a la derecha).
    let (s45, c45) = (-45.0f32).to_radians().sin_cos();
    let (nx, nz) = (x * c45 + z * s45, -x * s45 + z * c45);
    x = nx;
    z = nz;
    // El original gira 30 en X; aqui el eje y de la GUI ya viene espejado, asi que
    // el giro equivalente es el contrario (si no, el cubo se ve desde abajo).
    let (s30, c30) = (-30.0f32).to_radians().sin_cos();
    let (ny, nz2) = (y * c30 - z * s30, y * s30 + z * c30);
    y = ny;
    z = nz2;
    x = x * 16.0 + sw - 16.0;
    y = y * 16.0 + 16.0;
    z *= 16.0;
    [
        2.0 * x / sw - 1.0,
        1.0 - 2.0 * y / sh,
        // La z del cubo (16 unidades de GUI) va de -24 a 24; se lleva a 0.5±0.24
        // para que ninguna cara se salga del rango y el orden por profundidad mande.
        0.5 + z * 0.01,
    ]
}

// Textura del atlas por cara de los bloques pintables. Son los ids de Tile tal
// como estan en rd-160052/rd-20090515 (los que usa el motor de legacy::tiles;
// se repite aqui para que este archivo siga sirviendo a los binarios viejos).
fn hud_textura(id: u8, cara: usize) -> u32 {
    match id {
        1 => 1,              // piedra
        2 if cara == 1 => 0, // cesped: hierba arriba
        2 if cara == 0 => 2, // cesped: tierra abajo
        2 => 3,              // cesped: lados con fleco
        3 => 2,              // tierra
        4 => 16,             // ladrillo
        5 => 4,              // madera
        6 => 15,             // arbusto
        _ => 0,
    }
}

// El arbusto no es un cubo: Bush.render dibuja dos planos cruzados de suelo a
// techo. El drawGui del original llama al mismo render, asi que en la esquina
// tiene que salir la planta y no una cara de bloque con textura de hojas.
fn hud_arbusto_vertices(width: u32, height: u32) -> Vec<Vertex> {
    let sh = 240.0f32;
    let sw = width as f32 * 240.0 / height.max(1) as f32;
    let tex = 15u32;
    let u0 = (tex % 16) as f32 / 16.0;
    let v0 = (tex / 16) as f32 / 16.0;
    let duv = 0.0624375f32;
    let mut out = Vec::with_capacity(2 * 2 * 6);
    for r in 0..2usize {
        let a = r as f32 * std::f32::consts::PI / 2.0 + std::f32::consts::FRAC_PI_4;
        let xa = a.sin() * 0.5;
        let za = a.cos() * 0.5;
        // Mismas coordenadas que Bush.render para el bloque en (-2, 0, 0).
        let (x0, x1) = (-2.0 + 0.5 - xa, -2.0 + 0.5 + xa);
        let (z0, z1) = (0.0 + 0.5 - za, 0.0 + 0.5 + za);
        let (y0, y1) = (0.0, 1.0);
        // Orden de Tile.renderFace/Bush.render: u1/v0, u0/v0, u0/v1, u1/v1.
        let quad = [
            ([x0, y1, z0], [u0 + duv, v0]),
            ([x1, y1, z1], [u0, v0]),
            ([x1, y0, z1], [u0, v0 + duv]),
            ([x0, y0, z0], [u0 + duv, v0 + duv]),
        ];
        let mut p = [[0.0f32; 3]; 4];
        let mut uv = [[0.0f32; 2]; 4];
        for (i, (pos, t)) in quad.iter().enumerate() {
            p[i] = hud_proyecta(*pos, sw, sh);
            uv[i] = *t;
        }
        // El original emite cada plano dos veces, en los dos sentidos, porque el
        // espejo de la GUI invierte la orientacion y hay back-face culling.
        for e in [0usize, 1, 2, 0, 2, 3, 0, 2, 1, 0, 3, 2] {
            out.push(Vertex { pos: p[e], uv: uv[e], shade: 1.0 });
        }
    }
    out
}

fn hud_cubo_vertices(id: u8, width: u32, height: u32) -> Vec<Vertex> {
    if id == 6 {
        return hud_arbusto_vertices(width, height);
    }
    let sh = 240.0f32;
    let sw = width as f32 * 240.0 / height.max(1) as f32;
    let duv = 0.0624375f32;
    // (factor de luz, cara, 4 vertices: x, y, z, u01, v01) tal cual los manda
    // Tile.renderFace para el bloque en (x=-2, y=0, z=0).
    let caras: [(f32, usize, [[f32; 5]; 4]); 6] = [
        (1.0, 0, [[-2.0, 0.0, 1.0, 0.0, 1.0], [-2.0, 0.0, 0.0, 0.0, 0.0], [-1.0, 0.0, 0.0, 1.0, 0.0], [-1.0, 0.0, 1.0, 1.0, 1.0]]),
        (1.0, 1, [[-1.0, 1.0, 1.0, 1.0, 1.0], [-1.0, 1.0, 0.0, 1.0, 0.0], [-2.0, 1.0, 0.0, 0.0, 0.0], [-2.0, 1.0, 1.0, 0.0, 1.0]]),
        (0.8, 2, [[-2.0, 1.0, 0.0, 1.0, 0.0], [-1.0, 1.0, 0.0, 0.0, 0.0], [-1.0, 0.0, 0.0, 0.0, 1.0], [-2.0, 0.0, 0.0, 1.0, 1.0]]),
        (0.8, 3, [[-2.0, 1.0, 1.0, 0.0, 0.0], [-2.0, 0.0, 1.0, 0.0, 1.0], [-1.0, 0.0, 1.0, 1.0, 1.0], [-1.0, 1.0, 1.0, 1.0, 0.0]]),
        (0.6, 4, [[-2.0, 1.0, 1.0, 1.0, 0.0], [-2.0, 1.0, 0.0, 0.0, 0.0], [-2.0, 0.0, 0.0, 0.0, 1.0], [-2.0, 0.0, 1.0, 1.0, 1.0]]),
        (0.6, 5, [[-1.0, 0.0, 1.0, 0.0, 1.0], [-1.0, 0.0, 0.0, 1.0, 1.0], [-1.0, 1.0, 0.0, 1.0, 0.0], [-1.0, 1.0, 1.0, 0.0, 0.0]]),
    ];
    let mut out = Vec::with_capacity(HUD_VERTS);
    for (factor, cara, vs) in caras.iter() {
        let tex = hud_textura(id, *cara);
        let u0 = (tex % 16) as f32 / 16.0;
        let v0 = (tex / 16) as f32 / 16.0;
        let mut p = [[0.0f32; 3]; 4];
        let mut uv = [[0.0f32; 2]; 4];
        for (i, v) in vs.iter().enumerate() {
            p[i] = hud_proyecta([v[0], v[1], v[2]], sw, sh);
            uv[i] = [u0 + v[3] * duv, v0 + v[4] * duv];
        }
        for e in [0usize, 1, 2, 0, 2, 3, 0, 2, 1, 0, 3, 2] {
            out.push(Vertex { pos: p[e], uv: uv[e], shade: *factor });
        }
    }
    out
}

pub struct Renderer {
    pub window: Arc<Window>,
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group: wgpu::BindGroup,
    pub uniform_buf: wgpu::Buffer,
    pub depth_tex: wgpu::Texture,
    pub depth_view: wgpu::TextureView,
    pub chunks: Vec<Option<Chunk>>,
    pub hit_pipeline: wgpu::RenderPipeline,
    pub hit_preview_pipeline: wgpu::RenderPipeline,
    pub hit_uniform: wgpu::Buffer,
    pub hit_bind_group: wgpu::BindGroup,
    pub ui_pipeline: wgpu::RenderPipeline,
    // HUD de Classic con la fuente default.gif del jar.
    pub hud_fuente: Option<([u8; 128], f32, f32)>,
    pub text_pipeline: wgpu::RenderPipeline,
    pub text_uniform: wgpu::Buffer,
    pub text_bind_group: wgpu::BindGroup,
    pub text_vbuf: wgpu::Buffer,
    pub text_font_bg: Option<wgpu::BindGroup>,
    pub text_verts: usize,
    pub ui_uniform: wgpu::Buffer,
    pub ui_bind_group: wgpu::BindGroup,
    pub ui_vbuf: wgpu::Buffer,
    pub hit_tex_bg: wgpu::BindGroup,
    pub hit_flat_bg: wgpu::BindGroup,
    pub hit_face_vbuf: wgpu::Buffer,
    // Modelos de personaje
    pub model_pipeline: wgpu::RenderPipeline,
    pub model_uniform: wgpu::Buffer,
    pub model_bind_group: wgpu::BindGroup,
    pub model_fog_bind_group: wgpu::BindGroup,
    pub model_tex_bgl: wgpu::BindGroupLayout,
    pub model_bg: Option<wgpu::BindGroup>,
    pub model_vbuf: wgpu::Buffer,
    // Particulas: buffer persistente que se reescribe cada frame (como el de
    // los modelos), en vez de crear buffers nuevos por frame.
    pub part_vbuf: wgpu::Buffer,
    pub part_fondo_vbuf: wgpu::Buffer,
    pub part_ibuf: wgpu::Buffer,
    pub part_count: u32,
    pub part_fondo_count: u32,
    // Densidad de la niebla del pase oscuro y escala del alfa del resaltado:
    // la familia nueva usa 0.06 y la mitad de alfa que rd-132211.
    pub fog_densidad: f32,
    pub hit_alpha_escala: f32,
    // Bloque que se colocaria con el clic: si esta, renderHit va texturado con
    // blending normal; si no, velo blanco aditivo (modo romper).
    pub hit_preview: Option<u8>,
    // Cruz de mira (la dibuja el juego que la quiera, como el original)
    pub mostrar_cruz: bool,
    /// Classic: lineas del HUD en coordenadas de la GUI (alto 240), como
    /// Minecraft.drawGui (version arriba y "N fps, N chunk updates").
    pub hud_lineas: Vec<(String, f32, f32)>,
    /// Coordenadas del extra F3 en las versiones sin HUD propio.
    pub coords: Option<String>,
    /// F2: copia el frame actual a PNG al terminar de dibujarlo.
    pub pedir_captura: bool,
    /// Aviso de captura realizada: `Some` mientras corre la animacion.
    pub toast_inicio: Option<std::time::Instant>,
    pub toast_vbuf: wgpu::Buffer,
    pub toast_verts: usize,
    pub toast_pipeline: wgpu::RenderPipeline,
    // Cubo del bloque elegido en la esquina (RubyDung.drawGui)
    pub hud_bloque: Option<u8>,
    pub hud_uniform: wgpu::Buffer,
    pub hud_bg: wgpu::BindGroup,
    pub hud_vbuf: wgpu::Buffer,
    // Caras de la capa 1 (con bloque encima): el original las dibuja aparte con
    // niebla exponencial densa y casi negra.
    pub chunks_fog: Vec<Option<Chunk>>,
    pub fog_uniform: wgpu::Buffer,
    pub fog_bg: wgpu::BindGroup,
}

pub struct Chunk {
    pub vbuf: wgpu::Buffer,
    pub ibuf: wgpu::Buffer,
    pub icount: u32,
}

#[repr(C)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
    fog_color: [f32; 4],
    camera_pos: [f32; 4],
    fog_density: f32,
    fog_exp: f32,
    _pad: [f32; 2],
}

impl Renderer {
    pub async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .unwrap();
        let caps = surface.get_capabilities(&adapter);
        // Sin sRGB para que el color de cielo (0.5, 0.8, 1.0) salga igual que en el GL original
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .unwrap_or(caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            // COPY_SRC deja a F2 copiar el frame a un PNG antes de presentarlo.
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            // El original no usa vsync (su bucle corre libre): sin este modo, el
            // FPS de las dos versiones no es comparable.
            present_mode: wgpu::PresentMode::AutoNoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER_WGSL.into()),
        });

        // El atlas puede ser el de otra version (familia B tiene 6 tiles distintos):
        // RD_ATLAS_VERSION elige de que jar se saca, por defecto rd-132211.
        let version_atlas = std::env::var("RD_ATLAS_VERSION").unwrap_or_else(|_| "rd-132211".to_string());
        // Prioridad: RD_ATLAS (ruta elegida a mano o detectada por el launcher) ->
        // jar de la version en %APPDATA% -> terrain.png del jar de rd-132211 ->
        // textura de reemplazo propia.
        let atlas_data = std::env::var("RD_ATLAS").ok()
            .and_then(|r| cargar_atlas_de_ruta(&r))
            .filter(|(w, h, _)| *w == ATLAS_SIZE as u32 && *h == ATLAS_SIZE as u32)
            .map(|(_, _, px)| px)
            .or_else(|| {
                cargar_png_del_jar(&version_atlas, "terrain.png")
                    .filter(|(w, h, _)| *w == ATLAS_SIZE as u32 && *h == ATLAS_SIZE as u32)
                    .map(|(_, _, px)| px)
            })
            // 0.0.3a, 0.0.9a y 0.0.10a no tienen jar propio: usan el de c0.0.11a, que es
            // la primera Classic conservada y de donde salen estas versiones.
            .or_else(|| {
                cargar_png_del_jar("c0.0.11a", "terrain.png")
                    .filter(|(w, h, _)| *w == ATLAS_SIZE as u32 && *h == ATLAS_SIZE as u32)
                    .map(|(_, _, px)| px)
            })
            .or_else(load_original_atlas)
            .unwrap_or_else(generate_atlas);
        // RD_ATLAS_DUMP=1 escribe el atlas que se esta usando, para comprobarlo
        // contra el terrain.png del jar (careo de texturas).
        if std::env::var_os("RD_ATLAS_DUMP").is_some() {
            let f = std::fs::File::create("atlas_port.png");
            if let Ok(f) = f {
                let mut enc = png::Encoder::new(std::io::BufWriter::new(f), ATLAS_SIZE as u32, ATLAS_SIZE as u32);
                enc.set_color(png::ColorType::Rgba);
                enc.set_depth(png::BitDepth::Eight);
                if let Ok(mut w) = enc.write_header() {
                    let _ = w.write_image_data(&atlas_data);
                }
            }
        }
        let atlas = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("atlas"),
            size: wgpu::Extent3d {
                width: ATLAS_SIZE as u32,
                height: ATLAS_SIZE as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &atlas,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &atlas_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(ATLAS_SIZE as u32 * 4),
                rows_per_image: Some(ATLAS_SIZE as u32),
            },
            wgpu::Extent3d {
                width: ATLAS_SIZE as u32,
                height: ATLAS_SIZE as u32,
                depth_or_array_layers: 1,
            },
        );
        let atlas_view = atlas.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniforms"),
            size: std::mem::size_of::<Uniforms>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bg"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: uniform_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&atlas_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pl"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Vertex::layout()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Greater,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_tex = depth;

        // Pipeline de resaltado (caja del bloque apuntado): cubo unitario + color plano
        let hit_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("hit shader"),
            source: wgpu::ShaderSource::Wgsl(HIT_SHADER.into()),
        });
        // El resaltado muestrea el atlas, asi que necesita la textura en su propio
        // grupo de bindings (el grupo 0 es solo el uniform de proyeccion y alfa).
        let hit_tex_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("hit tex bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let hit_tex_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("hit tex bg"),
            layout: &hit_tex_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&atlas_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
        });
        // renderHit del original va SIN textura (glDisable(GL_TEXTURE_2D) antes de
        // dibujarlo): es un blanco plano. Para no tocar el shader se le da un
        // texel blanco de 1x1 en lugar del atlas.
        let hit_blanco = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("hit blanco"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &hit_blanco,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[255, 255, 255, 255],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        let hit_blanco_view = hit_blanco.create_view(&wgpu::TextureViewDescriptor::default());
        let hit_flat_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("hit flat bg"),
            layout: &hit_tex_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&hit_blanco_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
        });
        // 6 vertices de la cara resaltada, reescritos cada frame
        let hit_face_vbuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("hit face vbuf"),
            size: (36 * std::mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // HUD: cubo del bloque elegido, con el atlas y su propia matriz
        let hud_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("hud uniform"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let hud_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("hud bg"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: hud_uniform.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&atlas_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
        });
        let fog_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fog uniform"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let fog_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fog bg"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: fog_uniform.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&atlas_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
        });
        let hud_vbuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("hud vbuf"),
            size: (HUD_VERTS * std::mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let hit_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("hit bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let hit_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("hit uniform"),
            size: 144,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let hit_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("hit bg"),
            layout: &hit_bgl,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: hit_uniform.as_entire_binding() }],
        });
        let hit_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("hit pipeline"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("hit pl"),
                bind_group_layouts: &[&hit_bgl, &hit_tex_bgl],
                push_constant_ranges: &[],
            })),
            vertex: wgpu::VertexState {
                module: &hit_shader,
                entry_point: Some("vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Vertex::layout()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &hit_shader,
                entry_point: Some("fs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    // Capa blanca normal: se mezcla hacia blanco, no se suma al
                    // color del bloque (eso lo encendia entero).
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::GreaterEqual,
                stencil: wgpu::StencilState::default(),
                // La capa ya va 1 mm por delante de la cara (hit_face_vertices),
                // asi que no hace falta depth bias.
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        // Preview de colocacion: mismo shader y geometria, pero blending normal
        // (GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA), descarte de caras traseras y
        // escritura de profundidad. Sin escribir profundidad se veian las caras
        // interiores del bloque (el cubo parecia hueco).
        let hit_preview_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("hit preview pipeline"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("hit preview pl"),
                bind_group_layouts: &[&hit_bgl, &hit_tex_bgl],
                push_constant_ranges: &[],
            })),
            vertex: wgpu::VertexState {
                module: &hit_shader,
                entry_point: Some("vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Vertex::layout()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &hit_shader,
                entry_point: Some("fs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: Some(wgpu::Face::Back),
                // Mismo sentido que la malla del terreno (y que las seis caras
                // del preview): las caras interiores del cubo se descartan.
                front_face: wgpu::FrontFace::Ccw,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                // El bloque del preview se tapa a si mismo: sin esto, las caras
                // del fondo se mezclaban sobre las de delante y parecia hueco.
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::GreaterEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        // ---- modelos de personaje (zombis) ----
        let model_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("model shader"),
            source: wgpu::ShaderSource::Wgsl(MODEL_SHADER.into()),
        });
        let model_tex_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("model tex bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let model_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("model bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let model_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("model uniform"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let model_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("model bg"),
            layout: &model_bgl,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: model_uniform.as_entire_binding() }],
        });
        let model_fog_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("model fog bg"),
            layout: &model_bgl,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: fog_uniform.as_entire_binding() }],
        });
        // 100 zombis x 6 cajas x 6 caras x 6 verticess = 21600 vertices de 5 floats
        let model_vbuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("model vbuf"),
            size: 4 * 1024 * 1024,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let model_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("model pipeline"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("model pl"),
                bind_group_layouts: &[&model_bgl, &model_tex_bgl],
                push_constant_ranges: &[],
            })),
            vertex: wgpu::VertexState {
                module: &model_shader,
                entry_point: Some("vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 20,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x3 },
                        wgpu::VertexAttribute { offset: 12, shader_location: 1, format: wgpu::VertexFormat::Float32x2 },
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &model_shader,
                entry_point: Some("fs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::GreaterEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // ---- overlay de FPS (F3) ----
        let ui_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ui shader"),
            source: wgpu::ShaderSource::Wgsl(UI_SHADER.into()),
        });
        let ui_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ui bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let ui_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ui uniform"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let ui_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ui bg"),
            layout: &ui_bgl,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: ui_uniform.as_entire_binding() }],
        });
        // Pipeline del HUD de Classic: glifos texturados de default.gif, con
        // mezcla alfa normal y sin tocar la profundidad (van siempre encima).
        let text_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("text shader"),
            source: wgpu::ShaderSource::Wgsl(TEXT_SHADER.into()),
        });
        let text_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("text uniform"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let text_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("text bg"),
            layout: &ui_bgl,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: text_uniform.as_entire_binding() }],
        });
        let text_vbuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("text vbuf"),
            size: (20000 * std::mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let text_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("text pipeline"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("text pl"),
                bind_group_layouts: &[&ui_bgl, &hit_tex_bgl],
                push_constant_ranges: &[],
            })),
            vertex: wgpu::VertexState {
                module: &text_shader,
                entry_point: Some("vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Vertex::layout()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &text_shader,
                entry_point: Some("fs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        // La fuente sale del jar de la version que se esta jugando.
        let fuente = cargar_gif_del_jar(&version_atlas, "/default.gif")
            .or_else(|| cargar_gif_del_jar(&version_atlas, "default.gif"))
            .or_else(|| cargar_gif_del_jar("c0.0.11a", "default.gif"));
        let hud_fuente = fuente.as_ref().map(|g| {
            (fuente_anchos(&g.rgba, g.width, g.height), g.width as f32, g.height as f32)
        });
        let text_font_bg = fuente.as_ref().map(|g| {
            let tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("font"),
                size: wgpu::Extent3d {
                    width: g.width as u32,
                    height: g.height as u32,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &tex,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &g.rgba,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(g.width as u32 * 4),
                    rows_per_image: Some(g.height as u32),
                },
                wgpu::Extent3d {
                    width: g.width as u32,
                    height: g.height as u32,
                    depth_or_array_layers: 1,
                },
            );
            let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("font bg"),
                layout: &hit_tex_bgl,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
                ],
            })
        });
        let ui_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ui pipeline"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("ui pl"),
                bind_group_layouts: &[&ui_bgl],
                push_constant_ranges: &[],
            })),
            vertex: wgpu::VertexState {
                module: &ui_shader,
                entry_point: Some("vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 12,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float32x3,
                    }],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &ui_shader,
                entry_point: Some("fs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            // El texto va siempre por encima de la escena: nunca falla el test ni
            // escribe profundidad (el pase tiene adjunto de depth, asi que el
            // pipeline debe declarar el mismo formato).
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        // Tarjeta de captura: color por vertice, sin bind groups.
        let ui_color_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ui color shader"),
            source: wgpu::ShaderSource::Wgsl(UI_COLOR_SHADER.into()),
        });
        let toast_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("toast pipeline"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("toast pl"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            })),
            vertex: wgpu::VertexState {
                module: &ui_color_shader,
                entry_point: Some("vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 28,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x3 },
                        wgpu::VertexAttribute { offset: 12, shader_location: 1, format: wgpu::VertexFormat::Float32x4 },
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &ui_color_shader,
                entry_point: Some("fs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        // Particulas: buffer de vertices reutilizable + indices precalculados
        let part_vbuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("part vbuf"),
            size: (MAX_PART_QUADS as usize * 4 * std::mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let part_fondo_vbuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("part fondo vbuf"),
            size: (MAX_PART_QUADS as usize * 4 * std::mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut pidx: Vec<u32> = Vec::with_capacity(MAX_PART_QUADS as usize * 12);
        for q in 0..MAX_PART_QUADS {
            let b: u32 = q * 4;
            pidx.extend_from_slice(&[b, b + 1, b + 2, b, b + 2, b + 3, b, b + 2, b + 1, b, b + 3, b + 2]);
        }
        let part_ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("part ibuf"),
            contents: unsafe {
                std::slice::from_raw_parts(
                    pidx.as_ptr() as *const u8,
                    pidx.len() * std::mem::size_of::<u32>(),
                )
            },
            usage: wgpu::BufferUsages::INDEX,
        });

        let ui_vbuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ui vbuf"),
            size: (UI_MAX_VERTS * 12) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let toast_vbuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("toast vbuf"),
            // La tarjeta no llega a 400 vertices; 4096 deja aire de sobra.
            size: 4096 * 28,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Renderer {
            window,
            surface,
            device,
            queue,
            config,
            pipeline,
            bind_group,
            uniform_buf,
            depth_tex,
            depth_view,
            chunks: Vec::new(),
            hit_pipeline,
            hit_preview_pipeline,
            hit_uniform,
            hit_bind_group,
            ui_pipeline,
            hud_fuente,
            text_pipeline,
            text_uniform,
            text_bind_group,
            text_vbuf,
            text_font_bg,
            text_verts: 0,
            ui_uniform,
            ui_bind_group,
            ui_vbuf,
            part_vbuf,
            part_fondo_vbuf,
            part_ibuf,
            part_count: 0,
            part_fondo_count: 0,
            fog_densidad: 0.2,
            hit_alpha_escala: 1.0,
            hit_preview: None,
            mostrar_cruz: false,
            hud_lineas: Vec::new(),
            coords: None,
            pedir_captura: false,
            toast_inicio: None,
            toast_vbuf,
            toast_verts: 0,
            toast_pipeline,
            hud_bloque: None,
            hud_uniform,
            hud_bg,
            hud_vbuf,
            chunks_fog: Vec::new(),
            fog_uniform,
            fog_bg,
            hit_tex_bg,
            hit_flat_bg,
            hit_face_vbuf,
            model_pipeline,
            model_uniform,
            model_bind_group,
            model_fog_bind_group,
            model_tex_bgl,
            model_bg: None,
            model_vbuf,
        }
    }

    // Sube la piel (char.png) que usan los modelos de personaje.
    pub fn pon_piel(&mut self, w: u32, h: u32, rgba: &[u8]) {
        let tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("piel"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        // El original usa GL_NEAREST (9728) para la piel
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("piel sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        self.model_bg = Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("piel bg"),
            layout: &self.model_tex_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
        }));
    }

    // Sube una malla a la GPU como chunk
    fn chunk_desde(&self, verts: &[Vertex], indices: &[u32]) -> Chunk {
        let vbuf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("chunk vbuf"),
            contents: unsafe {
                std::slice::from_raw_parts(
                    verts.as_ptr() as *const u8,
                    verts.len() * std::mem::size_of::<Vertex>(),
                )
            },
            usage: wgpu::BufferUsages::VERTEX,
        });
        let ibuf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("chunk ibuf"),
            contents: unsafe {
                std::slice::from_raw_parts(
                    indices.as_ptr() as *const u8,
                    indices.len() * std::mem::size_of::<u32>(),
                )
            },
            usage: wgpu::BufferUsages::INDEX,
        });
        Chunk { vbuf, ibuf, icount: indices.len() as u32 }
    }

    fn chunk_opcional(&self, verts: &[Vertex], indices: &[u32]) -> Option<Chunk> {
        if indices.is_empty() {
            None
        } else {
            Some(self.chunk_desde(verts, indices))
        }
    }

    pub fn build_chunks(&mut self, level: &Level) {
        self.chunks.clear();
        self.chunks_fog.clear();
        let nx = crate::world::WIDTH / CHUNK;
        let ny = crate::world::HEIGHT / CHUNK;
        for cy in 0..ny {
            for cx in 0..nx {
                for capa in [0u8, 1] {
                    let (verts, indices) = build_chunk_mesh(level, cx, cy, capa);
                    let c = self.chunk_opcional(&verts, &indices);
                    if capa == 0 {
                        self.chunks.push(c);
                    } else {
                        self.chunks_fog.push(c);
                    }
                }
            }
        }
    }

    pub fn rebuild_chunk(&mut self, level: &Level, cx: usize, cy: usize) {
        let nx = crate::world::WIDTH / CHUNK;
        let i = cy * nx + cx;
        for capa in [0u8, 1] {
            let (verts, indices) = build_chunk_mesh(level, cx, cy, capa);
            let c = self.chunk_opcional(&verts, &indices);
            if capa == 0 {
                if i < self.chunks.len() {
                    self.chunks[i] = c;
                }
            } else if i < self.chunks_fog.len() {
                self.chunks_fog[i] = c;
            }
        }
    }

    // Sube mallas ya construidas (sirve para versiones con otro Level). Las dos
    // listas van en el mismo orden de chunks; un chunk sin caras queda como None
    // para que el indice de `actualizar_chunk` siga cuadrando.
    pub fn subir_chunks(
        &mut self,
        capa0: Vec<(Vec<Vertex>, Vec<u32>)>,
        capa1: Vec<(Vec<Vertex>, Vec<u32>)>,
    ) {
        let a: Vec<Option<Chunk>> = capa0
            .iter()
            .map(|(v, i)| self.chunk_opcional(v, i))
            .collect();
        let b: Vec<Option<Chunk>> = capa1
            .iter()
            .map(|(v, i)| self.chunk_opcional(v, i))
            .collect();
        self.chunks = a;
        self.chunks_fog = b;
    }

    // Sustituye las dos capas de un unico chunk sin re-subir el mundo entero.
    pub fn actualizar_chunk(
        &mut self,
        k: usize,
        capa0: (Vec<Vertex>, Vec<u32>),
        capa1: (Vec<Vertex>, Vec<u32>),
    ) {
        if k >= self.chunks.len() || k >= self.chunks_fog.len() {
            return;
        }
        let c0 = self.chunk_opcional(&capa0.0, &capa0.1);
        let c1 = self.chunk_opcional(&capa1.0, &capa1.1);
        self.chunks[k] = c0;
        self.chunks_fog[k] = c1;
    }

    // Particulas: reescribe el buffer persistente. Los indices ya estan
    // precalculados (12 por quad, doble cara).
    pub fn set_particulas(&mut self, lit: &[Vertex], fondo: &[Vertex]) {
        self.part_count = subir_particulas(&self.queue, &self.part_vbuf, lit);
        self.part_fondo_count = subir_particulas(&self.queue, &self.part_fondo_vbuf, fondo);
    }

    pub fn render(&mut self, eye: [f32; 3], yaw: f32, pitch: f32, hit: Option<((i32, i32, i32), (i32, i32, i32))>, fps: Option<u32>, modelos: &[[f32; 5]], modelos_fondo: &[[f32; 5]]) {
        // El original hace glTranslatef(0,0,-0.3) antes de rotar. Eso NO adelanta
        // la camara: la deja 0.3 bloques DETRAS del ojo (con yaw 0, en +z). Se
        // comprobo moviendo la camara del Java 0.6 y viendo que el zombi coincide
        // pixel a pixel con el port. Antes se sumaba 0.3 hacia delante y todo el
        // mundo salia 0.6 bloques desplazado respecto al original.
        let (sy_c, cy_c) = yaw.sin_cos();
        let (sp_c, cp_c) = pitch.sin_cos();
        let eye = [
            eye[0] - sy_c * cp_c * 0.3,
            eye[1] - sp_c * 0.3,
            eye[2] + cy_c * cp_c * 0.3,
        ];
        // El mundo mide 256x256x64: la diagonal maxima ronda 370 bloques. Un
        // far de 512 deja margen y gasta mucha mas precision de profundidad que
        // 1000, que es lo que hacia parpadear caras casi coplanares al moverse.
        let view_proj = mat_mul(&perspective(70.0f32.to_radians(), self.config.width as f32 / self.config.height as f32, 0.05, 512.0), &look_at(eye, yaw, pitch));
        // Pasada de niebla de rd-132211: GL_EXP, densidad 0.2, color (14,11,10).
        let fu = Uniforms {
            view_proj,
            fog_color: [14.0 / 255.0, 11.0 / 255.0, 10.0 / 255.0, 1.0],
            camera_pos: [eye[0], eye[1], eye[2], 1.0],
            fog_density: self.fog_densidad,
            fog_exp: 1.0,
            _pad: [0.0; 2],
        };
        self.queue.write_buffer(&self.fog_uniform, 0, unsafe {
            std::slice::from_raw_parts(&fu as *const Uniforms as *const u8, std::mem::size_of::<Uniforms>())
        });
        let u = Uniforms {
            view_proj,
            fog_color: [0.5, 0.8, 1.0, 1.0],
            camera_pos: [eye[0], eye[1], eye[2], 1.0],
            fog_density: 0.0,
            fog_exp: 0.0,
            _pad: [0.0; 2],
        };
        self.queue.write_buffer(&self.uniform_buf, 0, unsafe {
            std::slice::from_raw_parts(&u as *const Uniforms as *const u8, std::mem::size_of::<Uniforms>())
        });
        // Overlay 2D (FPS con F3 y cruz de mira): geometria ya en clip space
        let mut ui_count = 0u32;
        let mut ui_verts: Vec<f32> = Vec::new();
        if let Some(f) = fps {
            let mut text = format!("FPS {}", f);
            if let Some(c) = &self.coords {
                text.push('\n');
                text.push_str(c);
            }
            ui_verts.extend(ui_text_quads(&text, self.config.width, self.config.height));
        }
        if self.mostrar_cruz {
            ui_verts.extend(cruz_quads(self.config.width, self.config.height));
        }
        // HUD de Classic: con la fuente del jar, sombra incluida. Si el jar no
        // tiene default.gif se cae a la fuente 5x7 del contador.
        self.text_verts = 0;
        if !self.hud_lineas.is_empty() {
            let mut con_fuente = false;
            if let (Some((anchos, fw, fh)), true) = (&self.hud_fuente, self.text_font_bg.is_some()) {
                let (w, h) = (self.config.width, self.config.height);
                let mut tv: Vec<Vertex> = Vec::new();
                for sombra in [true, false] {
                    for (texto, x, y) in &self.hud_lineas {
                        fuente_quads(texto, *x, *y, w, h, *fw, *fh, anchos, sombra, &mut tv);
                    }
                }
                if tv.len() <= 20000 {
                    self.queue.write_buffer(&self.text_vbuf, 0, unsafe {
                        std::slice::from_raw_parts(
                            tv.as_ptr() as *const u8,
                            tv.len() * std::mem::size_of::<Vertex>(),
                        )
                    });
                    let blanco = [1.0f32, 1.0, 1.0, 1.0];
                    self.queue.write_buffer(&self.text_uniform, 0, unsafe {
                        std::slice::from_raw_parts(blanco.as_ptr() as *const u8, 16)
                    });
                    self.text_verts = tv.len();
                    con_fuente = true;
                }
            }
            if !con_fuente {
                for (texto, x, y) in &self.hud_lineas {
                    ui_verts.extend(ui_text_gui(texto, self.config.width, self.config.height, *x, *y));
                }
            }
        }
        if !ui_verts.is_empty() {
            let n = (ui_verts.len() / 3) as u32;
            if (n as usize) <= UI_MAX_VERTS {
                self.queue.write_buffer(&self.ui_vbuf, 0, unsafe {
                    std::slice::from_raw_parts(ui_verts.as_ptr() as *const u8, ui_verts.len() * 4)
                });
                let color = [1.0f32, 1.0, 1.0, 1.0];
                self.queue.write_buffer(&self.ui_uniform, 0, unsafe {
                    std::slice::from_raw_parts(color.as_ptr() as *const u8, 16)
                });
                ui_count = n;
            }
        }
        // Aviso de captura: la geometria se escribe aqui y se dibuja en el pase
        // del HUD, encima del cubo y del texto.
        self.toast_verts = 0;
        if let Some(inicio) = self.toast_inicio {
            let edad = inicio.elapsed().as_secs_f32();
            if edad >= TOAST_DURACION {
                self.toast_inicio = None;
            } else {
                // Classic dibuja el cubo del bloque en la esquina: ahi la
                // tarjeta baja para no taparlo. En el resto va pegada arriba.
                let y_base = if self.hud_bloque.is_some() { 88.0 } else { 14.0 };
                let verts = toast_quads(edad, self.config.width, self.config.height, y_base);
                if !verts.is_empty() && verts.len() / 7 <= 4096 {
                    self.queue.write_buffer(&self.toast_vbuf, 0, unsafe {
                        std::slice::from_raw_parts(verts.as_ptr() as *const u8, verts.len() * 4)
                    });
                    self.toast_verts = verts.len() / 7;
                }
            }
        }
        // HUD: cubo del bloque elegido (el drawGui del original)
        let hud_count = match self.hud_bloque {
            Some(id) => {
                let verts = hud_cubo_vertices(id, self.config.width, self.config.height);
                self.queue.write_buffer(&self.hud_vbuf, 0, unsafe {
                    std::slice::from_raw_parts(
                        verts.as_ptr() as *const u8,
                        verts.len() * std::mem::size_of::<Vertex>(),
                    )
                });
                verts.len() as u32
            }
            None => 0,
        };
        // Modelos: los claros van antes de la capa 1 y los oscuros dentro de
        // ella, como el bucle de zombis del original. Los dos comparten buffer
        // de vertices; el uniform de los oscuros es el mismo de la niebla.
        let max_verts = 4 * 1024 * 1024 / 20;
        let lit_n = modelos.len().min(max_verts);
        let fondo_n = modelos_fondo.len().min(max_verts - lit_n);
        if self.model_bg.is_some() && lit_n + fondo_n > 0 {
            if lit_n > 0 {
                self.queue.write_buffer(&self.model_vbuf, 0, unsafe {
                    std::slice::from_raw_parts(modelos.as_ptr() as *const u8, lit_n * 5 * 4)
                });
            }
            if fondo_n > 0 {
                self.queue.write_buffer(&self.model_vbuf, (lit_n * 5 * 4) as u64, unsafe {
                    std::slice::from_raw_parts(modelos_fondo.as_ptr() as *const u8, fondo_n * 5 * 4)
                });
            }
            let mu = Uniforms {
                view_proj,
                fog_color: [0.5, 0.8, 1.0, 1.0],
                camera_pos: [eye[0], eye[1], eye[2], 1.0],
                fog_density: 0.0,
                fog_exp: 0.0,
                _pad: [0.0; 2],
            };
            self.queue.write_buffer(&self.model_uniform, 0, unsafe {
                std::slice::from_raw_parts(&mu as *const Uniforms as *const u8, std::mem::size_of::<Uniforms>())
            });
        }
        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(_) => return,
        };
        // Si el frame real cambió de tamaño (HiDPI/resize), recrear el depth buffer
        let sz = frame.texture.size();
        if sz.width != self.depth_tex.size().width || sz.height != self.depth_tex.size().height {
            self.depth_tex = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("depth"),
                size: sz,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            self.depth_view = self.depth_tex.create_view(&wgpu::TextureViewDescriptor::default());
        }
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("enc") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.5, g: 0.8, b: 1.0, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            for c in self.chunks.iter().flatten() {
                if c.icount == 0 { continue; }
                pass.set_vertex_buffer(0, c.vbuf.slice(..));
                pass.set_index_buffer(c.ibuf.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..c.icount, 0, 0..1);
            }
            // Zombis a plena luz, como el primer bucle de Zombie.render.
            if lit_n > 0 {
                if let Some(bg) = &self.model_bg {
                    pass.set_pipeline(&self.model_pipeline);
                    pass.set_bind_group(0, &self.model_bind_group, &[]);
                    pass.set_bind_group(1, bg, &[]);
                    pass.set_vertex_buffer(0, self.model_vbuf.slice(..));
                    pass.draw(0..lit_n as u32, 0..1);
                }
            }
            // Particulas a plena luz (antes del cambio de niebla).
            if self.part_count > 0 {
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &self.bind_group, &[]);
                pass.set_vertex_buffer(0, self.part_vbuf.slice(..));
                pass.set_index_buffer(self.part_ibuf.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..self.part_count, 0, 0..1);
            }
            // Capa 1: las caras con bloque encima, con la niebla del original.
            if !self.chunks_fog.is_empty() || self.part_fondo_count > 0 || fondo_n > 0 {
                // Los zombis claros dejan el pipeline de modelos activo. Sin
                // volver al del terreno, esta capa se dibujaba con el pipeline
                // de modelos y el bind group de niebla no le encajaba (wgpu
                // abortaba el juego).
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &self.fog_bg, &[]);
                for c in self.chunks_fog.iter().flatten() {
                    if c.icount == 0 { continue; }
                    pass.set_vertex_buffer(0, c.vbuf.slice(..));
                    pass.set_index_buffer(c.ibuf.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..c.icount, 0, 0..1);
                }
                if fondo_n > 0 {
                    if let Some(bg) = &self.model_bg {
                        pass.set_pipeline(&self.model_pipeline);
                        pass.set_bind_group(0, &self.model_fog_bind_group, &[]);
                        pass.set_bind_group(1, bg, &[]);
                        pass.set_vertex_buffer(0, self.model_vbuf.slice(..));
                        pass.draw(lit_n as u32..(lit_n + fondo_n) as u32, 0..1);
                    }
                }
                if self.part_fondo_count > 0 {
                    pass.set_pipeline(&self.pipeline);
                    pass.set_vertex_buffer(0, self.part_fondo_vbuf.slice(..));
                    pass.set_index_buffer(self.part_ibuf.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..self.part_fondo_count, 0, 0..1);
                }
                pass.set_bind_group(0, &self.bind_group, &[]);
            }
            // Resaltado del bloque apuntado: en modo romper, una capa blanca en
            // la cara que se mira; en modo construir, el preview del bloque entero.
            if let Some(((hx, hy, hz), n)) = hit {
                #[repr(C)]
                struct HitU {
                    view_proj: [[f32; 4]; 4],
                    model: [[f32; 4]; 4],
                    color: [f32; 4],
                }
                // Modo romper: rd-132211 usa 0.4 + 0.2*sin(ms/100) (alfa 0.2..0.6);
                // la familia nueva envuelve la misma expresion en *0.5 (0.1..0.3).
                // Modo construir: brillo 0.6..1.0 (sin/100) y alfa 0.3..0.7
                // (sin/200), sin la escala del velo.
                let ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as f64;
                let preview = self.hit_preview;
                let color = match preview {
                    Some(_) => {
                        let br = ((ms / 100.0).sin() * 0.2 + 0.8) as f32;
                        let alpha = ((ms / 200.0).sin() * 0.2 + 0.5) as f32;
                        [br, br, br, alpha]
                    }
                    None => {
                        let alpha = (((ms / 100.0).sin() * 0.2 + 0.4) as f32)
                            * self.hit_alpha_escala;
                        [1.0, 1.0, 1.0, alpha]
                    }
                };
                // Los vertices van ya en coordenadas de mundo: modelo identidad.
                let ident = [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [0.0, 0.0, 0.0, 1.0],
                ];
                let hu = HitU { view_proj, model: ident, color };
                self.queue.write_buffer(&self.hit_uniform, 0, unsafe {
                    std::slice::from_raw_parts(&hu as *const HitU as *const u8, std::mem::size_of::<HitU>())
                });
                let verts = match preview {
                    Some(id) => hit_vertices((hx, hy, hz), Some(id)),
                    None => hit_face_vertices((hx, hy, hz), n).to_vec(),
                };
                self.queue.write_buffer(&self.hit_face_vbuf, 0, unsafe {
                    std::slice::from_raw_parts(verts.as_ptr() as *const u8, std::mem::size_of_val(&verts[..]))
                });
                if preview.is_some() {
                    pass.set_pipeline(&self.hit_preview_pipeline);
                    pass.set_bind_group(1, &self.hit_tex_bg, &[]);
                } else {
                    pass.set_pipeline(&self.hit_pipeline);
                    pass.set_bind_group(1, &self.hit_flat_bg, &[]);
                }
                pass.set_bind_group(0, &self.hit_bind_group, &[]);
                pass.set_vertex_buffer(0, self.hit_face_vbuf.slice(..));
                pass.draw(0..verts.len() as u32, 0..1);
            }
            if ui_count > 0 {
                pass.set_pipeline(&self.ui_pipeline);
                pass.set_bind_group(0, &self.ui_bind_group, &[]);
                pass.set_vertex_buffer(0, self.ui_vbuf.slice(..));
                pass.draw(0..ui_count, 0..1);
            }
        }
        // El original limpia el z-buffer antes de la GUI (drawGui), asi que el
        // cubo va en su propio pase con la profundidad limpia y el color encima.
        if hud_count > 0 || self.text_verts > 0 || self.toast_verts > 0 {
            let ident = [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ];
            let hu = Uniforms {
                view_proj: ident,
                fog_color: [0.5, 0.8, 1.0, 1.0],
                camera_pos: [0.0; 4],
                fog_density: 0.0,
                fog_exp: 0.0,
                _pad: [0.0; 2],
            };
            self.queue.write_buffer(&self.hud_uniform, 0, unsafe {
                std::slice::from_raw_parts(
                    &hu as *const Uniforms as *const u8,
                    std::mem::size_of::<Uniforms>(),
                )
            });
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("hud pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            if hud_count > 0 {
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &self.hud_bg, &[]);
                pass.set_vertex_buffer(0, self.hud_vbuf.slice(..));
                pass.draw(0..hud_count, 0..1);
            }
            if self.text_verts > 0 {
                if let Some(bg) = &self.text_font_bg {
                    pass.set_pipeline(&self.text_pipeline);
                    pass.set_bind_group(0, &self.text_bind_group, &[]);
                    pass.set_bind_group(1, bg, &[]);
                    pass.set_vertex_buffer(0, self.text_vbuf.slice(..));
                    pass.draw(0..self.text_verts as u32, 0..1);
                }
            }
            if self.toast_verts > 0 {
                pass.set_pipeline(&self.toast_pipeline);
                pass.set_vertex_buffer(0, self.toast_vbuf.slice(..));
                pass.draw(0..self.toast_verts as u32, 0..1);
            }
        }
        // F2: copiar el frame a un buffer antes de presentarlo. La lectura se
        // hace en el mismo thread porque el juego no vuelve a dibujar hasta
        // guardar el PNG; es un fotograma puntual, no una grabacion.
        let mut captura: Option<(wgpu::Buffer, u32, u32, u32)> = None;
        if self.pedir_captura {
            self.pedir_captura = false;
            let w = sz.width;
            let h = sz.height;
            let padded = (w * 4 + 255) & !255;
            let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("screenshot"),
                size: padded as u64 * h as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: &frame.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(padded),
                        rows_per_image: Some(h),
                    },
                },
                wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
            );
            captura = Some((buffer, w, h, padded));
        }
        self.queue.submit(Some(encoder.finish()));
        if let Some((buffer, w, h, padded)) = captura {
            let slice = buffer.slice(..);
            let (tx, rx) = std::sync::mpsc::channel();
            slice.map_async(wgpu::MapMode::Read, move |r| {
                let _ = tx.send(r);
            });
            let _ = self.device.poll(wgpu::PollType::Wait);
            let _ = rx.recv();
            let guardada = {
                let data = slice.get_mapped_range();
                guardar_captura(w, h, padded, &data, self.config.format)
            };
            buffer.unmap();
            if guardada {
                self.toast_inicio = Some(std::time::Instant::now());
            }
        }
        frame.present();
    }
}

/// Strip the row padding the GPU needs, turn BGRA into RGBA when needed and
/// write a PNG under `screenshots/` in the world folder.
fn guardar_captura(w: u32, h: u32, padded: u32, data: &[u8], format: wgpu::TextureFormat) -> bool {
    let bgra = matches!(format, wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb);
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for row in data.chunks(padded as usize) {
        for px in row[..(w * 4) as usize].chunks_exact(4) {
            if bgra {
                rgba.extend_from_slice(&[px[2], px[1], px[0], px[3]]);
            } else {
                rgba.extend_from_slice(px);
            }
        }
    }
    let Ok(dir) = std::env::current_dir().map(|d| d.join("screenshots")) else { return false };
    if std::fs::create_dir_all(&dir).is_err() {
        return false;
    }
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let path = dir.join(format!("shot-{ms}.png"));
    let Ok(f) = std::fs::File::create(&path) else { return false };
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    match enc.write_header() {
        Ok(mut writer) => writer.write_image_data(&rgba).is_ok(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod toast_tests {
    use super::*;

    #[test]
    fn la_tarjeta_de_captura_entra_y_se_va() {
        let dentro = toast_quads(0.6, 1024, 768, 14.0);
        assert!(!dentro.is_empty());
        // Cada vertice son 7 floats (posicion y color).
        assert_eq!(dentro.len() % 7, 0);
        assert!(toast_quads(TOAST_DURACION + 0.1, 1024, 768, 14.0).is_empty());
        // En una ventana diminuta no cabe: no se dibuja.
        assert!(toast_quads(0.6, 320, 240, 14.0).is_empty());
    }

    #[test]
    fn la_tarjeta_entra_desde_la_derecha() {
        let al_empezar = toast_quads(0.02, 1024, 768, 14.0);
        let al_final = toast_quads(0.30, 1024, 768, 14.0);
        assert!(!al_empezar.is_empty() && !al_final.is_empty());
        // El primer vertice es la esquina superior izquierda del panel.
        assert!(al_empezar[0] > al_final[0]);
    }
}

/// Initial window size chosen by the launcher for the current instance. Each
/// side is independent; a missing or insane value keeps the size that build
/// used originally (640x480 for c0.0.9a, 1024x768 for the rest). The smoke/photo
/// harnesses keep their own fixed size.
pub fn tamano_inicial(defecto: (u32, u32)) -> (u32, u32) {
    let side = |key: &str| {
        std::env::var(key)
            .ok()
            .and_then(|v| v.trim().parse::<u32>().ok())
            .filter(|v| (320..=7680).contains(v))
    };
    (
        side("RD_WIDTH").unwrap_or(defecto.0),
        side("RD_HEIGHT").unwrap_or(defecto.1),
    )
}

// Proyeccion con profundidad INVERSA (reversed-Z): near -> 1, far -> 0.
// Con float32 la precision deja de desplomarse con la distancia, que es justo
// donde el resaltado (1 mm delante de la cara) se peleaba con el bloque.
// Requiere depth_compare Greater/GreaterEqual y limpiar la profundidad a 0.0.
pub fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> [[f32; 4]; 4] {
    let f = 1.0 / (fov_y / 2.0).tan();
    [
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, near / (far - near), -1.0],
        [0.0, 0.0, near * far / (far - near), 0.0],
    ]
}

pub fn look_at(eye: [f32; 3], yaw: f32, pitch: f32) -> [[f32; 4]; 4] {
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    let fwd = [sy * cp, sp, -cy * cp];
    // El eje derecha solo depende del yaw: [cos, 0, sin]. Calculado con el
    // producto vectorial se degenera al mirar justo arriba o abajo, y limitar
    // el pitch a 1.55 rad desalineaba la camara del pick, que si llega a 90.
    let right = [cy, 0.0, sy];
    let upv = cross(right, fwd);
    [
        [right[0], upv[0], -fwd[0], 0.0],
        [right[1], upv[1], -fwd[1], 0.0],
        [right[2], upv[2], -fwd[2], 0.0],
        [-dot(right, eye), -dot(upv, eye), dot(fwd, eye), 1.0],
    ]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub fn mat_mul(a: &[[f32; 4]; 4], b: &[[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut r = [[0.0f32; 4]; 4];
    for c in 0..4 {
        for row in 0..4 {
            r[c][row] = a[0][row] * b[c][0] + a[1][row] * b[c][1] + a[2][row] * b[c][2] + a[3][row] * b[c][3];
        }
    }
    r
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub uv: [f32; 2],
    pub shade: f32,
}

impl Vertex {
    pub const ATTRIBUTES: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32];

    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

const MAX_PART_QUADS: u32 = 512;

pub const ATLAS_SIZE: usize = 256;
pub const TILE: usize = 16;
pub const TILES_PER_ROW: usize = ATLAS_SIZE / TILE;

fn hash2(x: usize, y: usize, seed: u32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x8da6b343)
        ^ (y as u32).wrapping_mul(0xd8163841)
        ^ seed.wrapping_mul(0xcb1ab1df);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b3c6d);
    h ^= h >> 12;
    h
}

// Carga el terrain.png ORIGINAL desde la propia instalacion local del juego
// (tu copia legitima de rd-132211). No se copia ningun asset al proyecto:
// solo si el archivo existe en tu PC se usa; si no, se usa el atlas procedural.
pub fn load_original_atlas() -> Option<Vec<u8>> {
    let appdata = std::env::var("APPDATA").ok()?;
    let jar = format!("{}/.minecraft/versions/rd-132211/rd-132211.jar", appdata);
    let data = std::fs::read(&jar).ok()?;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(data)).ok()?;
    let mut entry = archive.by_name("terrain.png").ok()?;
    let mut png_bytes = Vec::new();
    std::io::Read::read_to_end(&mut entry, &mut png_bytes).ok()?;
    let mut decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let (w, h) = (reader.info().width as usize, reader.info().height as usize);
    // Tras EXPAND la salida es 8-bit RGB/RGBA/gray: reserva 4 bytes/pixel de sobra
    let mut raw = vec![0u8; w * h * 4];
    let out = reader.next_frame(&mut raw).ok()?;
    let (w, h) = (out.width as usize, out.height as usize);
    let bpp = out.buffer_size() / (w * h);
    let mut rgba = vec![0u8; w * h * 4];
    let ct = out.color_type;
    for i in 0..w * h {
        let (r, g, b, a) = match ct {
            png::ColorType::Grayscale => { let v = raw[i]; (v, v, v, 255) }
            png::ColorType::GrayscaleAlpha => { let v = raw[i * 2]; (v, v, v, raw[i * 2 + 1]) }
            png::ColorType::Rgb => { (raw[i * 3], raw[i * 3 + 1], raw[i * 3 + 2], 255) }
            png::ColorType::Rgba => { (raw[i * 4], raw[i * 4 + 1], raw[i * 4 + 2], raw[i * 4 + 3]) }
            _ => (0, 0, 0, 255),
        };
        let _ = bpp;
        rgba[i * 4] = r;
        rgba[i * 4 + 1] = g;
        rgba[i * 4 + 2] = b;
        rgba[i * 4 + 3] = a;
    }
    if w == 256 && h == 256 { Some(rgba) } else { None }
}

// Atlas desde una ruta concreta (RD_ATLAS): vale un .jar del que se saca
// terrain.png, o un .png suelto. Asi el jugador puede decir de donde coger las
// texturas si las tiene en una instalacion que no es la de %APPDATA%.
pub fn cargar_atlas_de_ruta(ruta: &str) -> Option<(u32, u32, Vec<u8>)> {
    let datos = std::fs::read(ruta).ok()?;
    let png_bytes = if datos.starts_with(b"PK") {
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(datos)).ok()?;
        let mut entry = archive.by_name("terrain.png").ok()?;
        let mut v = Vec::new();
        std::io::Read::read_to_end(&mut entry, &mut v).ok()?;
        v
    } else {
        datos
    };
    let mut decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let (w, h) = (reader.info().width as usize, reader.info().height as usize);
    let mut raw = vec![0u8; w * h * 4];
    let out = reader.next_frame(&mut raw).ok()?;
    let (w, h) = (out.width as usize, out.height as usize);
    let mut rgba = vec![0u8; w * h * 4];
    for i in 0..w * h {
        let (r, g, b, a) = match out.color_type {
            png::ColorType::Grayscale => { let v = raw[i]; (v, v, v, 255) }
            png::ColorType::GrayscaleAlpha => { let v = raw[i * 2]; (v, v, v, raw[i * 2 + 1]) }
            png::ColorType::Rgb => (raw[i * 3], raw[i * 3 + 1], raw[i * 3 + 2], 255),
            png::ColorType::Rgba => (raw[i * 4], raw[i * 4 + 1], raw[i * 4 + 2], raw[i * 4 + 3]),
            _ => (0, 0, 0, 255),
        };
        rgba[i * 4] = r;
        rgba[i * 4 + 1] = g;
        rgba[i * 4 + 2] = b;
        rgba[i * 4 + 3] = a;
    }
    Some((w as u32, h as u32, rgba))
}

// Saca un PNG de un jar de una version concreta (instalacion local del usuario)
// y lo devuelve como RGBA de 8 bits. Sirve para terrain.png y para char.png.
// ------------------------------------------------------- GIF (default.gif)
// El HUD de Classic usa la fuente default.gif del jar. Para no meter una
// dependencia solo por esto, aqui va un decodificador GIF minimo (paleta, LZW,
// transparencia y entrelazado) que basta para ese fichero.

pub struct Gif {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}

fn gif_lzw(data: &[u8], min_code: u8) -> Vec<u8> {
    let clear = 1u16 << min_code;
    let eoi = clear + 1;
    let mut dict: Vec<Vec<u8>> = Vec::new();
    let reset = |dict: &mut Vec<Vec<u8>>| {
        dict.clear();
        for i in 0..clear {
            dict.push(vec![i as u8]);
        }
        dict.push(Vec::new());
        dict.push(Vec::new());
    };
    reset(&mut dict);
    let mut size = min_code + 1;
    let mut out = Vec::new();
    let mut bit = 0usize;
    let mut prev: Option<Vec<u8>> = None;
    while bit + size as usize <= data.len() * 8 {
        let mut code = 0u16;
        for i in 0..size as usize {
            let b = (data[(bit + i) / 8] >> ((bit + i) % 8)) & 1;
            code |= (b as u16) << i;
        }
        bit += size as usize;
        if code == clear {
            reset(&mut dict);
            size = min_code + 1;
            prev = None;
            continue;
        }
        if code == eoi {
            break;
        }
        let entry: Vec<u8> = if (code as usize) < dict.len() {
            dict[code as usize].clone()
        } else if let Some(p) = &prev {
            let mut e = p.clone();
            e.push(p[0]);
            e
        } else {
            break;
        };
        out.extend_from_slice(&entry);
        if let Some(p) = prev {
            let mut e = p.clone();
            e.push(entry[0]);
            dict.push(e);
            if dict.len() == (1usize << size) && size < 12 {
                size += 1;
            }
        }
        prev = Some(entry);
    }
    out
}

fn le16(d: &[u8], p: usize) -> Option<usize> {
    Some(u16::from_le_bytes([*d.get(p)?, *d.get(p + 1)?]) as usize)
}

pub fn decode_gif(data: &[u8]) -> Option<Gif> {
    if data.len() < 13 || &data[0..3] != b"GIF" {
        return None;
    }
    let w = le16(data, 6)?;
    let h = le16(data, 8)?;
    let packed = data[10];
    let mut p = 13;
    let mut palette = vec![[0u8; 3]; 256];
    if packed & 0x80 != 0 {
        let n = 1usize << ((packed & 7) + 1);
        for slot in palette.iter_mut().take(n) {
            if p + 3 > data.len() {
                return None;
            }
            *slot = [data[p], data[p + 1], data[p + 2]];
            p += 3;
        }
    }
    let mut transparent: Option<u8> = None;
    loop {
        let b = *data.get(p)?;
        p += 1;
        match b {
            0x21 => {
                let label = *data.get(p)?;
                p += 1;
                if label == 0xF9 {
                    let size = *data.get(p)? as usize;
                    p += 1;
                    if size >= 4 {
                        let flags = data[p];
                        transparent = if flags & 1 != 0 { Some(data[p + 3]) } else { None };
                    }
                    p += size;
                }
                loop {
                    let s = *data.get(p)? as usize;
                    p += 1;
                    if s == 0 {
                        break;
                    }
                    p += s;
                }
            }
            0x2C => {
                let left = le16(data, p)?;
                let top = le16(data, p + 2)?;
                let iw = le16(data, p + 4)?;
                let ih = le16(data, p + 6)?;
                let flags = data[p + 8];
                p += 9;
                if flags & 0x80 != 0 {
                    let n = 1usize << ((flags & 7) + 1);
                    for slot in palette.iter_mut().take(n) {
                        if p + 3 > data.len() {
                            return None;
                        }
                        *slot = [data[p], data[p + 1], data[p + 2]];
                        p += 3;
                    }
                }
                let interlace = flags & 0x40 != 0;
                let min_code = *data.get(p)?;
                p += 1;
                let mut lzw = Vec::new();
                loop {
                    let s = *data.get(p)? as usize;
                    p += 1;
                    if s == 0 {
                        break;
                    }
                    lzw.extend_from_slice(data.get(p..p + s)?);
                    p += s;
                }
                let idx = gif_lzw(&lzw, min_code);
                let mut rgba = vec![0u8; w * h * 4];
                // Orden de filas del GIF entrelazado (passes 0..3).
                let mut filas: Vec<usize> = if interlace {
                    let mut f = Vec::with_capacity(ih);
                    for (inicio, paso) in [(0, 8), (4, 8), (2, 4), (1, 2)] {
                        let mut y = inicio;
                        while y < ih {
                            f.push(y);
                            y += paso;
                        }
                    }
                    f
                } else {
                    (0..ih).collect()
                };
                for y in 0..ih {
                    let fy = filas.get(y).copied().unwrap_or(y);
                    for x in 0..iw {
                        let ci = idx.get(y * iw + x).copied().unwrap_or(0) as usize;
                        let (cx, cy) = (left + x, top + fy);
                        if cx >= w || cy >= h {
                            continue;
                        }
                        let c = palette[ci.min(255)];
                        let o = (cy * w + cx) * 4;
                        rgba[o] = c[0];
                        rgba[o + 1] = c[1];
                        rgba[o + 2] = c[2];
                        rgba[o + 3] = if transparent == Some(ci as u8) { 0 } else { 255 };
                    }
                }
                return Some(Gif { width: w, height: h, rgba });
            }
            0x3B => return None,
            _ => return None,
        }
    }
}

pub fn cargar_gif_del_jar(version: &str, nombre: &str) -> Option<Gif> {
    let appdata = std::env::var("APPDATA").ok()?;
    let jar = format!("{}/.minecraft/versions/{v}/{v}.jar", appdata, v = version);
    let data = std::fs::read(&jar).ok()?;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(data)).ok()?;
    let mut entry = archive.by_name(nombre).ok()?;
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut entry, &mut bytes).ok()?;
    decode_gif(&bytes)
}

pub fn cargar_png_del_jar(version: &str, nombre: &str) -> Option<(u32, u32, Vec<u8>)> {
    let appdata = std::env::var("APPDATA").ok()?;
    let jar = format!("{}/.minecraft/versions/{v}/{v}.jar", appdata, v = version);
    let data = std::fs::read(&jar).ok()?;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(data)).ok()?;
    let mut entry = archive.by_name(nombre).ok()?;
    let mut png_bytes = Vec::new();
    std::io::Read::read_to_end(&mut entry, &mut png_bytes).ok()?;
    let mut decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let (w, h) = (reader.info().width as usize, reader.info().height as usize);
    let mut raw = vec![0u8; w * h * 4];
    let out = reader.next_frame(&mut raw).ok()?;
    let (w, h) = (out.width as usize, out.height as usize);
    let mut rgba = vec![0u8; w * h * 4];
    for i in 0..w * h {
        let (r, g, b, a) = match out.color_type {
            png::ColorType::Grayscale => { let v = raw[i]; (v, v, v, 255) }
            png::ColorType::GrayscaleAlpha => { let v = raw[i * 2]; (v, v, v, raw[i * 2 + 1]) }
            png::ColorType::Rgb => (raw[i * 3], raw[i * 3 + 1], raw[i * 3 + 2], 255),
            png::ColorType::Rgba => (raw[i * 4], raw[i * 4 + 1], raw[i * 4 + 2], raw[i * 4 + 3]),
            _ => (0, 0, 0, 255),
        };
        rgba[i * 4] = r;
        rgba[i * 4 + 1] = g;
        rgba[i * 4 + 2] = b;
        rgba[i * 4 + 3] = a;
    }
    Some((w as u32, h as u32, rgba))
}

// Textura de REEMPLAZO: la usa el juego cuando el equipo no tiene ninguna copia
// de la version original (no es de Mojang: los colores y el ruido son nuestros).
// Es jugable y se ve decente en vez del tablero magenta de depuracion, que queda
// solo si se pide con RD_ATLAS_DEBUG=1.
pub fn generate_atlas() -> Vec<u8> {
    // 0 = hierba, 1 = piedra (los dos que usa rd-132211); el resto, tonos variados
    // para que las otras versiones no salgan de un solo color.
    const BASE: [(i32, i32, i32); 16] = [
        (95, 159, 53),
        (128, 128, 128),
        (134, 96, 67),
        (163, 122, 84),
        (150, 62, 52),
        (196, 196, 196),
        (86, 122, 176),
        (206, 186, 74),
        (74, 122, 58),
        (170, 170, 170),
        (58, 58, 58),
        (120, 90, 150),
        (150, 110, 90),
        (90, 150, 160),
        (200, 140, 120),
        (110, 110, 120),
    ];
    let debug = std::env::var_os("RD_ATLAS_DEBUG").is_some();
    let mut data = vec![0u8; ATLAS_SIZE * ATLAS_SIZE * 4];
    for ty in 0..TILES_PER_ROW {
        for tx in 0..TILES_PER_ROW {
            let tile = ty * TILES_PER_ROW + tx;
            let (br, bg, bb) = if tile < 16 { BASE[tile] } else { (110, 110, 120) };
            for py in 0..TILE {
                for px in 0..TILE {
                    let x = tx * TILE + px;
                    let y = ty * TILE + py;
                    let i = (y * ATLAS_SIZE + x) * 4;
                    if debug {
                        let (r, g, b) = if (px / 4 + py / 4) % 2 == 0 { (0, 0, 0) } else { (255, 0, 255) };
                        data[i] = r;
                        data[i + 1] = g;
                        data[i + 2] = b;
                        data[i + 3] = 255;
                        continue;
                    }
                    // ruido fijo por texel: el mundo se ve con textura y no liso
                    let h = (hash2(x, y, 0x9e37_79b9) % 41) as i32 - 20;
                    data[i] = (br + h).clamp(0, 255) as u8;
                    data[i + 1] = (bg + h).clamp(0, 255) as u8;
                    data[i + 2] = (bb + h).clamp(0, 255) as u8;
                    data[i + 3] = 255;
                }
            }
        }
    }
    data
}

fn subir_particulas(queue: &wgpu::Queue, buf: &wgpu::Buffer, verts: &[Vertex]) -> u32 {
    let n = verts.len().min(MAX_PART_QUADS as usize * 4);
    let quads = (n / 4) as u32;
    if quads == 0 {
        return 0;
    }
    queue.write_buffer(buf, 0, unsafe {
        std::slice::from_raw_parts(verts.as_ptr() as *const u8, n * std::mem::size_of::<Vertex>())
    });
    quads * 12
}

// Genera la malla de un chunk de 16x16 (x,y) x 64 (z). Solo caras visibles.
// Coordenadas de render: X=x, Y=z (arriba), Z=y.
// capa 0 = caras a plena luz, capa 1 = caras con bloque solido encima en su
// columna (Tile.render del original las parte con la comparacion `br == c`).
pub fn build_chunk_mesh(level: &Level, cx: usize, cy: usize, capa: u8) -> (Vec<Vertex>, Vec<u32>) {
    let mut verts: Vec<Vertex> = Vec::new();
    let mut idx: Vec<u32> = Vec::new();
    // (dx, dy, dz del vecino, factor de cara, posicion donde se muestrea getBrightness,
    //  4 esquinas en (x, z, y) locales del bloque)
    // Factores de Tile.render del original: el eje vertical vale 1.0 y los dos
    // horizontales 0.8 y 0.6. En el original el eje vertical es la "y"; aqui es la z.
    let faces: [(i32, i32, i32, f32, (i32, i32, i32), [[f32; 3]; 4]); 6] = [
        (0, 0, 1, 1.0, (0, 0, 0), [[0.0, 1.0, 0.0], [0.0, 1.0, 1.0], [1.0, 1.0, 1.0], [1.0, 1.0, 0.0]]),
        (0, 0, -1, 1.0, (0, 0, -1), [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 1.0], [0.0, 0.0, 1.0]]),
        (1, 0, 0, 0.6, (1, 0, 0), [[1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [1.0, 1.0, 1.0], [1.0, 0.0, 1.0]]),
        (-1, 0, 0, 0.6, (-1, 0, 0), [[0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 1.0], [0.0, 1.0, 0.0]]),
        (0, 1, 0, 0.8, (0, 1, 0), [[0.0, 0.0, 1.0], [1.0, 0.0, 1.0], [1.0, 1.0, 1.0], [0.0, 1.0, 1.0]]),
        (0, -1, 0, 0.8, (0, -1, 0), [[0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0, 0.0], [1.0, 0.0, 0.0]]),
    ];
    let uvs = [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]];
    for lx in 0..CHUNK {
        for ly in 0..CHUNK {
            let x = cx * CHUNK + lx;
            let y = cy * CHUNK + ly;
            for z in 0..crate::world::DEPTH as i32 {
                let b = level.get(x as i32, y as i32, z);
                if b == AIR {
                    continue;
                }
                // Tile por altura como el Chunk original: en rd-132211/rd-132328
                // es verde solo la cota fija depth*2/3 y gris todo lo demas (por
                // eso colocar encima no cambia el bloque de abajo); en rd-131655,
                // que no tuvo jar, manda la superficie de cada columna. En el
                // terrain.png original: tile 0 = verde, tile 1 = gris.
                let tile = level.tile_texture(x as i32, y as i32, z);
                // UV como el original: 0.0624375 (= 16/256 menos un pelo) en vez del
                // 1/16 exacto. Ese margen evita que, al muestrear, el borde de la cara
                // agarre la ultima columna del tile vecino del atlas (la roca gris),
                // que es lo que ensuciaba el pasto a distancia.
                let u0 = tile / 16.0;
                let u1 = u0 + 0.0624375;
                let v0 = 0.0;
                let v1 = 0.0624375;
                for (face, f) in faces.iter().enumerate() {
                    let (dx, dy, dz, factor, (ox, oy, oz), corners) = *f;
                    if level.get(x as i32 + dx, y as i32 + dy, z + dz) != AIR {
                        continue;
                    }
                    let brillo = level.brightness(x as i32 + ox, y as i32 + oy, z + oz);
                    if (capa == 0) != (brillo >= 1.0) {
                        continue;
                    }
                    let shade = factor * brillo;
                    let base = verts.len() as u32;
                    for (ci, c) in corners.iter().enumerate() {
                        let u = if uvs[ci][0] > 0.5 { u1 } else { u0 };
                        let v = if uvs[ci][1] > 0.5 { v1 } else { v0 };
                        verts.push(Vertex {
                            pos: [x as f32 + c[0], z as f32 + c[1], y as f32 + c[2]],
                            uv: [u, v],
                            shade,
                        });
                    }
                    let _ = face;
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
    fn mirar_en_vertical_conserva_el_rumbo() {
        let yaw = 0.7f32;
        for pitch in [-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2] {
            let m = look_at([0.0; 3], yaw, pitch);
            // La columna 0 es el eje derecha: (cos yaw, 0, sin yaw).
            assert!((m[0][0] - yaw.cos()).abs() < 1e-5);
            assert!(m[1][0].abs() < 1e-5);
            assert!((m[2][0] - yaw.sin()).abs() < 1e-5);
        }
    }
}

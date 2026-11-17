# rd-132211: niebla exponencial en la segunda capa (las caras con bloque encima).
# El original hace dos pasadas: capa 0 sin niebla, capa 1 con GL_EXP, densidad 0.2
# y color (14,11,10). Aqui se separan las mallas por capa y se les aplica la
# niebla correspondiente en el shader.
import io
import sys

p = 'src/render.rs'
s = io.open(p, encoding='utf-8').read()


def rep(viejo, nuevo, n=1):
    global s
    if viejo not in s:
        print('NO ENCONTRADO:', viejo.strip().splitlines()[0][:70])
        sys.exit(1)
    s = s.replace(viejo, nuevo, n)


# ---- shader: bandera de niebla exponencial + distancia de ojo ----
rep(
    """struct Uniforms {
    view_proj: mat4x4<f32>,
    fog_color: vec4<f32>,
    camera_pos: vec4<f32>,
    fog_density: f32,
};""",
    """struct Uniforms {
    view_proj: mat4x4<f32>,
    fog_color: vec4<f32>,
    camera_pos: vec4<f32>,
    fog_density: f32,
    // 1.0 = GL_EXP (rd-132211); 0.0 = la cuadratica de la familia nueva
    fog_exp: f32,
};""",
)

rep(
    """    @location(2) world: vec3<f32>,
};""",
    """    // Distancia en el eje de la vista (|z_eye|): es la que usa glFog, no la
    // distancia euclidea.
    @location(2) fogz: f32,
};""",
)

rep(
    """    var o: VOut;
    o.clip = u.view_proj * vec4<f32>(pos, 1.0);
    o.uv = uv;
    o.shade = shade;
    o.world = pos;
    return o;""",
    """    var o: VOut;
    let clip = u.view_proj * vec4<f32>(pos, 1.0);
    o.clip = clip;
    o.uv = uv;
    o.shade = shade;
    o.fogz = clip.w;
    return o;""",
)

rep(
    """    let c = textureSample(tex, samp, v.uv);
    let dist = distance(v.world, u.camera_pos.xyz);
    let fog = 1.0 - exp(-dist * dist * u.fog_density * u.fog_density);
    let rgb = mix(c.rgb * v.shade, u.fog_color.rgb, clamp(fog, 0.0, 1.0));""",
    """    let c = textureSample(tex, samp, v.uv);
    let dist = max(v.fogz, 0.0);
    var fog = 1.0 - exp(-dist * dist * u.fog_density * u.fog_density);
    if (u.fog_exp > 0.5) {
        fog = 1.0 - exp(-dist * u.fog_density);
    }
    let rgb = mix(c.rgb * v.shade, u.fog_color.rgb, clamp(fog, 0.0, 1.0));""",
)

# ---- struct de uniforms en Rust ----
rep(
    """    fog_density: f32,
    _pad: [f32; 3],
}""",
    """    fog_density: f32,
    fog_exp: f32,
    _pad: [f32; 2],
}""",
)
s = s.replace("""            fog_density: 0.0,
            _pad: [0.0; 3],""", """            fog_density: 0.0,
            fog_exp: 0.0,
            _pad: [0.0; 2],""")
s = s.replace("""                fog_density: 0.0,
                _pad: [0.0; 3],""", """                fog_density: 0.0,
                fog_exp: 0.0,
                _pad: [0.0; 2],""")

# ---- campo y bind group de la pasada con niebla ----
rep(
    """    pub hud_vbuf: wgpu::Buffer,""",
    """    pub hud_vbuf: wgpu::Buffer,
    // Caras de la capa 1 (con bloque encima): el original las dibuja aparte con
    // niebla exponencial densa y casi negra.
    pub chunks_fog: Vec<Chunk>,
    pub fog_uniform: wgpu::Buffer,
    pub fog_bg: wgpu::BindGroup,""",
)

rep(
    """        let hud_vbuf = device.create_buffer(&wgpu::BufferDescriptor {""",
    """        let fog_uniform = device.create_buffer(&wgpu::BufferDescriptor {
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
        let hud_vbuf = device.create_buffer(&wgpu::BufferDescriptor {""",
)

rep(
    """            hud_vbuf,
            hit_tex_bg,""",
    """            hud_vbuf,
            chunks_fog: Vec::new(),
            fog_uniform,
            fog_bg,
            hit_tex_bg,""",
)

# ---- escritura del uniform de niebla y dibujo de esos chunks ----
rep(
    """        let u = Uniforms {
            view_proj,""",
    """        // Pasada de niebla de rd-132211: GL_EXP, densidad 0.2, color (14,11,10).
        let fu = Uniforms {
            view_proj,
            fog_color: [14.0 / 255.0, 11.0 / 255.0, 10.0 / 255.0, 1.0],
            camera_pos: [eye[0], eye[1], eye[2], 1.0],
            fog_density: 0.2,
            fog_exp: 1.0,
            _pad: [0.0; 2],
        };
        self.queue.write_buffer(&self.fog_uniform, 0, unsafe {
            std::slice::from_raw_parts(&fu as *const Uniforms as *const u8, std::mem::size_of::<Uniforms>())
        });
        let u = Uniforms {
            view_proj,""",
)

rep(
    """            if self.part_count > 0 {""",
    """            // Capa 1: las caras con bloque encima, con la niebla del original.
            if !self.chunks_fog.is_empty() {
                pass.set_bind_group(0, &self.fog_bg, &[]);
                for c in &self.chunks_fog {
                    if c.icount == 0 { continue; }
                    pass.set_vertex_buffer(0, c.vbuf.slice(..));
                    pass.set_index_buffer(c.ibuf.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..c.icount, 0, 0..1);
                }
                pass.set_bind_group(0, &self.bind_group, &[]);
            }
            if self.part_count > 0 {""",
)

# ---- mallado por capas ----
rep(
    """pub fn build_chunk_mesh(level: &Level, cx: usize, cy: usize) -> (Vec<Vertex>, Vec<u32>) {""",
    """// capa 0 = caras a plena luz, capa 1 = caras con bloque solido encima en su
// columna (Tile.render del original las parte con la comparacion `br == c`).
pub fn build_chunk_mesh(level: &Level, cx: usize, cy: usize, capa: u8) -> (Vec<Vertex>, Vec<u32>) {""",
)

rep(
    """                    let shade =
                        factor * level.brightness(x as i32 + ox, y as i32 + oy, z + oz);""",
    """                    let brillo = level.brightness(x as i32 + ox, y as i32 + oy, z + oz);
                    if (capa == 0) != (brillo >= 1.0) {
                        continue;
                    }
                    let shade = factor * brillo;""",
)

rep(
    """    pub fn build_chunks(&mut self, level: &Level) {
        self.chunks.clear();
        let nx = crate::world::WIDTH / CHUNK;
        let ny = crate::world::HEIGHT / CHUNK;
        for cy in 0..ny {
            for cx in 0..nx {
                let (verts, indices) = build_chunk_mesh(level, cx, cy);
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
                self.chunks.push(Chunk { vbuf, ibuf, icount: indices.len() as u32 });
            }
        }
    }""",
    """    // Sube una malla a la GPU como chunk
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

    pub fn build_chunks(&mut self, level: &Level) {
        self.chunks.clear();
        self.chunks_fog.clear();
        let nx = crate::world::WIDTH / CHUNK;
        let ny = crate::world::HEIGHT / CHUNK;
        for cy in 0..ny {
            for cx in 0..nx {
                for capa in [0u8, 1] {
                    let (verts, indices) = build_chunk_mesh(level, cx, cy, capa);
                    let c = self.chunk_desde(&verts, &indices);
                    if capa == 0 {
                        self.chunks.push(c);
                    } else {
                        self.chunks_fog.push(c);
                    }
                }
            }
        }
    }""",
)

rep(
    """    pub fn rebuild_chunk(&mut self, level: &Level, cx: usize, cy: usize) {
        let nx = crate::world::WIDTH / CHUNK;
        let (verts, indices) = build_chunk_mesh(level, cx, cy);
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
        self.chunks[cy * nx + cx] = Chunk { vbuf, ibuf, icount: indices.len() as u32 };
    }""",
    """    pub fn rebuild_chunk(&mut self, level: &Level, cx: usize, cy: usize) {
        let nx = crate::world::WIDTH / CHUNK;
        let i = cy * nx + cx;
        for capa in [0u8, 1] {
            let (verts, indices) = build_chunk_mesh(level, cx, cy, capa);
            let c = self.chunk_desde(&verts, &indices);
            if capa == 0 {
                if i < self.chunks.len() {
                    self.chunks[i] = c;
                }
            } else if i < self.chunks_fog.len() {
                self.chunks_fog[i] = c;
            }
        }
    }""",
)

rep(
    """    pub fn subir_chunks(&mut self, mallas: Vec<(Vec<Vertex>, Vec<u32>)>) {
        self.chunks.clear();""",
    """    pub fn subir_chunks(&mut self, mallas: Vec<(Vec<Vertex>, Vec<u32>)>) {
        self.chunks.clear();
        self.chunks_fog.clear();""",
)

io.open(p, 'w', encoding='utf-8', newline='').write(s)
print('render.rs parcheado')

# ---- el brillo de las caras tapadas vuelve a ser 0.8: la niebla ya hace el resto ----
p = 'src/world.rs'
t = io.open(p, encoding='utf-8').read()
viejo = """// Brillo de una cara que tiene bloque solido por encima en su columna. El bytecode
// dice 0.8, pero el valor medido en el original es 0.52 (ver Level::brightness).
pub const SHADED: f32 = 0.52;"""
nuevo = """// Brillo de una cara que tiene bloque solido por encima en su columna (0.8 en el
// bytecode de Level.getBrightness). Antes se usaba 0.52, que era el resultado de
// medir una captura a ~2 bloques; ese 0.52 es 0.8 * exp(-0.2 * 2.18), o sea el
// brillo por la niebla del segundo pase. Ahora la niebla se aplica de verdad
// (GL_EXP, densidad 0.2) y aqui queda el 0.8 limpio.
pub const SHADED: f32 = 0.8;"""
if viejo not in t:
    print('NO ENCONTRADO: SHADED')
    sys.exit(1)
io.open(p, 'w', encoding='utf-8', newline='').write(t.replace(viejo, nuevo, 1))
print('world.rs parcheado')

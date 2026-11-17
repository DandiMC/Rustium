# Añade el cubo del bloque elegido (RubyDung.drawGui) al HUD del port.
import io
import sys

p = 'src/render.rs'
s = io.open(p, encoding='utf-8').read()
antes = s


def rep(viejo, nuevo):
    global s
    if viejo not in s:
        print('NO ENCONTRADO:', viejo.strip().splitlines()[0][:70])
        sys.exit(1)
    s = s.replace(viejo, nuevo, 1)


rep(
    "    // Cruz de mira (la dibuja el juego que la quiera, como el original)\n"
    "    pub mostrar_cruz: bool,",
    "    // Cruz de mira (la dibuja el juego que la quiera, como el original)\n"
    "    pub mostrar_cruz: bool,\n"
    "    // Cubo del bloque elegido en la esquina (RubyDung.drawGui)\n"
    "    pub hud_bloque: Option<u8>,\n"
    "    pub hud_uniform: wgpu::Buffer,\n"
    "    pub hud_bg: wgpu::BindGroup,\n"
    "    pub hud_vbuf: wgpu::Buffer,",
)

rep(
    '        let hit_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {\n'
    '            label: Some("hit bgl"),',
    '        // HUD: cubo del bloque elegido, con el atlas y su propia matriz\n'
    '        let hud_uniform = device.create_buffer(&wgpu::BufferDescriptor {\n'
    '            label: Some("hud uniform"),\n'
    '            size: std::mem::size_of::<Uniforms>() as u64,\n'
    '            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,\n'
    '            mapped_at_creation: false,\n'
    '        });\n'
    '        let hud_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {\n'
    '            label: Some("hud bg"),\n'
    '            layout: &bind_group_layout,\n'
    '            entries: &[\n'
    '                wgpu::BindGroupEntry { binding: 0, resource: hud_uniform.as_entire_binding() },\n'
    '                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&atlas_view) },\n'
    '                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sampler) },\n'
    '            ],\n'
    '        });\n'
    '        let hud_vbuf = device.create_buffer(&wgpu::BufferDescriptor {\n'
    '            label: Some("hud vbuf"),\n'
    '            size: (HUD_VERTS * std::mem::size_of::<Vertex>()) as u64,\n'
    '            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,\n'
    '            mapped_at_creation: false,\n'
    '        });\n'
    '        let hit_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {\n'
    '            label: Some("hit bgl"),',
)

rep(
    "            mostrar_cruz: false,\n            hit_tex_bg,",
    "            mostrar_cruz: false,\n"
    "            hud_bloque: None,\n"
    "            hud_uniform,\n"
    "            hud_bg,\n"
    "            hud_vbuf,\n"
    "            hit_tex_bg,",
)

cabecera_hud = '''// Cubo del bloque elegido, como en RubyDung.drawGui: en la ortografica de 240 de
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
    let (s45, c45) = 45.0f32.to_radians().sin_cos();
    let (nx, nz) = (x * c45 + z * s45, -x * s45 + z * c45);
    x = nx;
    z = nz;
    let (s30, c30) = 30.0f32.to_radians().sin_cos();
    let (ny, nz2) = (y * c30 - z * s30, y * s30 + z * c30);
    y = ny;
    z = nz2;
    x = x * 16.0 + sw - 16.0;
    y = y * 16.0 + 16.0;
    z *= 16.0;
    [
        2.0 * x / sw - 1.0,
        1.0 - 2.0 * y / sh,
        (0.5 + z / 32.0).clamp(0.01, 0.99),
    ]
}

fn hud_cubo_vertices(id: u8, width: u32, height: u32) -> Vec<Vertex> {
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
        let tex = crate::legacy::tiles::textura(id, *cara);
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

pub struct Renderer {'''

rep("pub struct Renderer {", cabecera_hud)

rep(
    "        let frame = match self.surface.get_current_texture() {",
    "        // HUD: cubo del bloque elegido (el drawGui del original)\n"
    "        let hud_count = match self.hud_bloque {\n"
    "            Some(id) => {\n"
    "                let verts = hud_cubo_vertices(id, self.config.width, self.config.height);\n"
    "                self.queue.write_buffer(&self.hud_vbuf, 0, unsafe {\n"
    "                    std::slice::from_raw_parts(\n"
    "                        verts.as_ptr() as *const u8,\n"
    "                        verts.len() * std::mem::size_of::<Vertex>(),\n"
    "                    )\n"
    "                });\n"
    "                verts.len() as u32\n"
    "            }\n"
    "            None => 0,\n"
    "        };\n"
    "        let frame = match self.surface.get_current_texture() {",
)

pase_hud = '''        // El original limpia el z-buffer antes de la GUI (drawGui), asi que el
        // cubo va en su propio pase con la profundidad limpia y el color encima.
        if hud_count > 0 {
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
                _pad: [0.0; 3],
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
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.hud_bg, &[]);
            pass.set_vertex_buffer(0, self.hud_vbuf.slice(..));
            pass.draw(0..hud_count, 0..1);
        }
        self.queue.submit(Some(encoder.finish()));
        frame.present();'''

rep(
    "        self.queue.submit(Some(encoder.finish()));\n        frame.present();",
    pase_hud,
)

io.open(p, 'w', encoding='utf-8', newline='').write(s)
print('render.rs parcheado:', s != antes)

p = 'src/legacy/juego.rs'
s = io.open(p, encoding='utf-8').read()
if 'hud_bloque' not in s:
    viejo = "            r.set_particulas(&pv);\n            r.render(eye, yaw, pitch, self.hit, fps, &mv);"
    nuevo = (
        "            r.set_particulas(&pv);\n"
        "            // El original siempre enseña el bloque elegido en la esquina\n"
        "            r.hud_bloque = Some(self.pintura);\n"
        "            r.render(eye, yaw, pitch, self.hit, fps, &mv);"
    )
    if viejo not in s:
        print('NO ENCONTRADO en juego.rs')
        sys.exit(1)
    s = s.replace(viejo, nuevo, 1)
    io.open(p, 'w', encoding='utf-8', newline='').write(s)
print('juego.rs parcheado:', 'hud_bloque' in io.open(p, encoding='utf-8').read())

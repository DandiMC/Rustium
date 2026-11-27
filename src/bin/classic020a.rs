// c0.0.20a: inventario con B, bloque de oro, dieciseis telas, dienteleones,
// rosas, setas, /solid y bedrock irrompible.

#[path = "../render.rs"]
mod render;
#[path = "../world.rs"]
mod world;
#[path = "../entity.rs"]
mod entity;
#[path = "../legacy/mod.rs"]
mod legacy;

#[path = "../crash.rs"]
mod crash;

fn main() {
    crash::install();
    legacy::juego::correr_clasico_014_familia("c0.0.20a");
}

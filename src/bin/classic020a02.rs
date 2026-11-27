// c0.0.20a_02: corrige la transparencia del agua y conserva el inventario,
// las telas, las plantas y el bedrock irrompible de 0.0.20a.

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
    legacy::juego::correr_clasico_014_familia("c0.0.20a_02");
}

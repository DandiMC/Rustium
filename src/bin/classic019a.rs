// c0.0.19a: cristal, esponja, bloque de oro, barra rapida de nueve bloques,
// animacion de liquidos y limite de 100 fps.

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
    legacy::juego::correr_clasico_014_familia("c0.0.19a");
}

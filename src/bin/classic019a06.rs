// c0.0.19a_06: build de correccion del temporizador de 0.0.19a. Mantiene el
// cristal, la esponja y la barra rapida, con el tick mas estable.

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
    legacy::juego::correr_clasico_014_familia("c0.0.19a_06");
}

// c0.0.17a: /setspawn, el registro de chat al escribir y la lista de jugadores
// con Tab. En solitario la lista muestra al unico jugador del mundo.

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
    legacy::juego::correr_clasico_014_familia("c0.0.17a");
}

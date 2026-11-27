// c0.0.15a_01: reupload del test multijugador. La wiki documenta mejoras de
// interpolacion del cliente; en el port, que juega en solitario, no cambia la
// partida. Comparte mundo y reglas con 0.0.15a.

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
    legacy::juego::correr_clasico_014_familia("c0.0.15a_01");
}

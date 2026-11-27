// c0.0.16a_02: el ultimo test multijugador. En el port conserva la consola de
// 0.0.16a y estrena la niebla indigo bajo el agua que documenta la wiki.

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
    legacy::juego::correr_clasico_014_familia("c0.0.16a_02");
}

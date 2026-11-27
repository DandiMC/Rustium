// 0.24_SURVIVAL_TEST: salud, flechas con Tab, creepers, cerdos, esqueletos y
// zombis, drops, puntos y muerte sin respawn automatico.

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
    legacy::juego::correr_clasico_014_familia("0.24_SURVIVAL_TEST");
}

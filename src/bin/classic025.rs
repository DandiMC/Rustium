// 0.25_SURVIVAL_TEST: guardado del mundo, mobs mas rapidos y con mas alcance,
// reaparicion de mobs, peleas entre monstruos, flechas moradas de esqueleto y
// puntos en pantalla. Ultima build de esta tanda.

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
    legacy::juego::correr_clasico_014_familia("0.25_SURVIVAL_TEST");
}

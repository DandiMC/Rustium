// c0.0.12a_03: el reupload esta archivado, aunque el upload original se
// perdio. La wiki lo describe como una version de correcciones, sobre todo de
// guardado y rendimiento, sin cambios de juego. Comparte el mundo y el motor de
// c0.0.12a.

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
    legacy::juego::correr_clasico_con("c0.0.12a_03", 1024, 768, true);
}

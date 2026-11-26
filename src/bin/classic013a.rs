// c0.0.13a: reconstruccion de la build de desarrollo que ofrece el launcher
// oficial (la release final esta perdida). Esa build usa un mundo plano de
// 32x512x64 con cuevas y lava subterranea, ventana de 854x480 y el formato de
// level.dat con cabecera de 0.0.13a. El guardado con cuenta de la release
// perdida no forma parte de esta build.

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
    legacy::juego::correr_clasico_con("c0.0.13a", 854, 480, true);
}

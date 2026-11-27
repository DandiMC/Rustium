// c0.0.23a: menu de opciones con Escape, controles configurables y distancia
// de dibujado ajustable. Es la ultima Classic de construccion.

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
    legacy::juego::correr_clasico_014_familia("c0.0.23a");
}

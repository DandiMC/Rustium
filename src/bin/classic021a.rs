// c0.0.21a: inventario con fondo gris, se puede andar y cambiar de casilla con
// el inventario abierto, auto-clic y mayor alcance de construccion.

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
    legacy::juego::correr_clasico_014_familia("c0.0.21a");
}

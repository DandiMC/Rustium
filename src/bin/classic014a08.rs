// c0.0.14a_08: ultima build de desarrollo antes del test multijugador.
// La copa de los arboles pasa a forma de cruz, la niebla se vuelve blanca y
// las superficies de agua y lava reciben sombra. Mantiene la ventana 854x480,
// el atlas de 0.0.14a y el selector de nueve bloques.

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
    legacy::juego::correr_clasico_014_familia("c0.0.14a_08");
}

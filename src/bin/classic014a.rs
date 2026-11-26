// c0.0.14a: la version que anadio arena, grava, las tres menas, troncos y
// hojas, los arboles, las nubes y la niebla de distancia lineal. Su jar esta
// perdido; los tiles nuevos se toman del jar de 0.0.14a_08, la ultima build de
// desarrollo que queda archivada, pero el comportamiento reconstruido es el de
// la 0.0.14a base: copa de arbol cuadrada y niebla azul lineal, sin los cambios
// exclusivos de _08 (copa en cruz, niebla blanca, sombras en agua/lava).
// Mantiene la ventana de 854x480 de 0.0.13a.

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
    legacy::juego::correr_clasico_014();
}

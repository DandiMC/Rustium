// c0.0.3a: reconstruccion. Su jar no se conserva, asi que se monta sobre el
// codigo de c0.0.11a con lo que la wiki documenta de 0.0.3a: es la version que
// volvio a traer las cuevas, una hora antes de 0.0.9a. Aquel 0.0.9a anadio el
// modo de construccion (boton derecho), el contador de fps y chunk updates, el
// velo blanco del bloque elegido y el cambio a 640x480; aqui no esta nada de
// eso. Corre a 1024x768, el izquierdo coloca, el derecho rompe y Escape cierra.

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
    legacy::juego::correr_clasico_reconstruida("c0.0.3a", false, true, false);
}

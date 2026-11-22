// c0.0.9a: reconstruccion. Su jar no se conserva (nunca se archivo), asi que se
// monta sobre el codigo de c0.0.11a con lo que la wiki documenta de 0.0.9a: el
// modo construir (ya presente), el contador de fps y chunk updates, el velo
// blanco por todas las caras, la resolucion 640x480 y Escape cerrando el juego.

#[path = "../render.rs"]
mod render;
#[path = "../world.rs"]
mod world;
#[path = "../entity.rs"]
mod entity;
#[path = "../legacy/mod.rs"]
mod legacy;

fn main() {
    legacy::juego::correr_clasico_con("c0.0.9a", 640, 480, false);
}

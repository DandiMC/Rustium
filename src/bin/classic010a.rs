// c0.0.10a: reconstruccion. Solo duro 4 minutos y su jar no se conserva; la
// wiki deja dos cosas claras frente a c0.0.9a: Escape suelta el raton en vez de
// cerrar el juego y se arreglo un cuelgue, mientras que el fallo de colocar y
// romper que traia se arreglo en c0.0.11a. Aqui va el motor de 0.0.11a con el
// Escape de 0.0.10a.

#[path = "../render.rs"]
mod render;
#[path = "../world.rs"]
mod world;
#[path = "../entity.rs"]
mod entity;
#[path = "../legacy/mod.rs"]
mod legacy;

fn main() {
    legacy::juego::correr_clasico_con("c0.0.10a", 640, 480, true);
}

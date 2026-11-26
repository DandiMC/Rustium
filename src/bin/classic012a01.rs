// c0.0.12a_01: reconstruccion. Su jar no se conserva; la wiki solo documenta
// un cambio pequeno: los mobs vuelven a reiniciarse al crear un nivel nuevo.
// Comparte el mundo y el motor de c0.0.12a.

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
    legacy::juego::correr_clasico_con("c0.0.12a_01", 1024, 768, true);
}

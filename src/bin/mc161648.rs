// mc-161648: reconstruccion de "Minecraft alpha" (16 de mayo de 2009, 16:48
// UTC). Su jar no se conserva. La wiki documenta que arreglo el retardo del
// raton y que es la ultima version sin numero de version en pantalla; el resto
// de cambios es desconocido. Sigue sin HUD, como las anteriores.

#[path = "../render.rs"]
mod render;
#[path = "../world.rs"]
mod world;
#[path = "../entity.rs"]
mod entity;
#[path = "../legacy/mod.rs"]
mod legacy;

fn main() {
    legacy::juego::correr_clasico_reconstruida("mc-161648", true, false, true);
}

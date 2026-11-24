// mc-161625: reconstruccion de "Minecraft alpha" (16 de mayo de 2009, 16:25
// UTC). Su jar no se conserva. La wiki documenta que arreglo las texturas y el
// invertido del raton; el resto de cambios es desconocido. Sigue sin numero de
// version en pantalla.

#[path = "../render.rs"]
mod render;
#[path = "../world.rs"]
mod world;
#[path = "../entity.rs"]
mod entity;
#[path = "../legacy/mod.rs"]
mod legacy;

fn main() {
    legacy::juego::correr_clasico_reconstruida("mc-161625", true, false, true);
}

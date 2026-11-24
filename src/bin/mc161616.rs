// mc-161616: reconstruccion de "Minecraft alpha" (16 de mayo de 2009, 16:16
// UTC). Su jar no se conserva. La wiki documenta que anadio la tecla Y para
// invertir el eje del raton y cambio el movimiento del raton; el resto de
// cambios es desconocido. Sigue sin numero de version en pantalla.

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
    legacy::juego::correr_clasico_reconstruida("mc-161616", true, false, true);
}

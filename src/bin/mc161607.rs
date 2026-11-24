// mc-161607: reconstruccion de "Minecraft alpha", la primera version que jugo
// alguien de fuera de Mojang (prueba privada del 16 de mayo de 2009, 16:07 UTC).
// Su jar no se conserva. La wiki solo documenta que Escape libera el raton en
// vez de cerrar; el resto de cambios es desconocido. Es anterior al numero de
// version en pantalla, al contador de fps y al modo de construccion de 0.0.9a,
// asi que va a 1024x768, el izquierdo coloca y el derecho rompe.

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
    legacy::juego::correr_clasico_reconstruida("mc-161607", true, false, false);
}

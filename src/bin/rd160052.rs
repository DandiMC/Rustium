// rd-160052: mismo motor que rd-20090515, sin el tile "Bush" (que el generador
// nunca coloca, asi que el comportamiento jugable es identico).

#[path = "../render.rs"]
mod render;
#[path = "../world.rs"]
mod world;
#[path = "../entity.rs"]
mod entity;
#[path = "../legacy/mod.rs"]
mod legacy;

fn main() {
    legacy::juego::correr("rd-160052");
}

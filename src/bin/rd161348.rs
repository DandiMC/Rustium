// rd-161348: el ultimo build pre-clasico. Su jar es identico al de rd-20090515
// (mismo sha1), asi que comparte motor; solo cambia el nombre que ensena el
// juego y el fichero de mundo que guarda.

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
    legacy::juego::correr("rd-161348");
}

// rd-20090515 (y rd-161348, el MISMO jar): primera version publica.
// Todo el motor vive en src/legacy/juego.rs, compartido con rd-160052.

#[path = "../render.rs"]
mod render;
#[path = "../world.rs"]
mod world;
#[path = "../entity.rs"]
mod entity;
#[path = "../legacy/mod.rs"]
mod legacy;

fn main() {
    legacy::juego::correr("rd-20090515");
}

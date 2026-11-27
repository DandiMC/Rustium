// c0.0.16a: primera build con los comandos de servidor /kick y /broadcast.
// En el port se juega en solitario, asi que ambos se adaptan a la consola de
// chat (T): /broadcast escribe el mensaje en el registro y /kick explica que
// no hay otros jugadores. El mundo sigue siendo el de 0.0.15a.

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
    legacy::juego::correr_clasico_014_familia("c0.0.16a");
}

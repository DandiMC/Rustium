// c0.0.18a_02: la fuente admite \, |, @ y $; la consola de chat ya acepta esos
// caracteres y el HUD los dibuja con la fuente del jar.

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
    legacy::juego::correr_clasico_014_familia("c0.0.18a_02");
}

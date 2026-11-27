// c0.0.22a: sonidos basicos, mute con M y lava mas lenta. El port sintetiza
// los efectos en memoria y no reproduce musica.

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
    legacy::juego::correr_clasico_014_familia("c0.0.22a");
}

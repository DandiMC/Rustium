// c0.0.18a: /teleport. En el port el comando acepta coordenadas (x, fondo,
// altura) porque no hay otros jugadores a los que teleportar.

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
    legacy::juego::correr_clasico_014_familia("c0.0.18a");
}

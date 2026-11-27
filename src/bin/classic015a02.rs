// c0.0.15a_02: segundo reupload del test multijugador. La wiki documenta la
// animacion de mobs y jugadores corregida; en el port no hay otros jugadores,
// asi que la partida es la de 0.0.15a.

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
    legacy::juego::correr_clasico_014_familia("c0.0.15a_02");
}

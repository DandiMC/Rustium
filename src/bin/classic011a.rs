// c0.0.11a: la primera Classic que se conserva. Mismo motor que la familia
// pre-clasica, pero el mundo sale de LevelGen con NoiseMap y los clics son los
// de Minecraft: el izquierdo rompe o pinta segun el modo, el derecho lo cambia,
// y el HUD enseña la version y los fps con los chunk updates.

#[path = "../render.rs"]
mod render;
#[path = "../world.rs"]
mod world;
#[path = "../entity.rs"]
mod entity;
#[path = "../legacy/mod.rs"]
mod legacy;

fn main() {
    legacy::juego::correr_clasico("c0.0.11a");
}

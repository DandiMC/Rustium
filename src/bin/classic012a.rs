// c0.0.12a: el primer Classic con agua, lava y bedrock. Reutiliza el motor de
// c0.0.11a, pero el mundo se genera con oceano, islas y lagos de lava; el agua
// y la lava se atraviesan y permiten nadar. El jar original esta perdido: las
// texturas base salen de c0.0.11a y los tres tiles nuevos se importan de
// c0.0.13a_03 si esta instalada, o los dibuja el port como fallback.

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
    legacy::juego::correr_clasico("c0.0.12a");
}

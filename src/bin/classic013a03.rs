// c0.0.13a_03: version oficial archivada en el launcher. Mantiene la ventana
// de 854x480 y el mundo clasico de 256x256x64 con liquidos; respecto a 0.0.12a
// la wiki documenta que los lagos de lava ya no aparecen por encima del nivel
// del mar. Usa el level.dat con cabecera de 0.0.13a y acepta los guardados
// crudos anteriores.

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
    legacy::juego::correr_clasico_con("c0.0.13a_03", 854, 480, true);
}

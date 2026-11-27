// c0.0.15a: primer test multijugador. En el port se juega en solitario, pero
// conserva los cambios de la build: la grava solo aparece bajo el agua, los
// arboles estrenan copa de roble pequeno y la lava fluye mas despacio y sale
// mas brillante. Mantiene la ventana 854x480 y el atlas de 0.0.14a.

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
    legacy::juego::correr_clasico_014_familia("c0.0.15a");
}

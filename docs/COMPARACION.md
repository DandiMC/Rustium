# Rustium: comparación build por build

**Estado: port experimental, todas las builds en beta.** Este documento no vende Rustium como una recreación fiel. Recoge, una por una, qué se conserva, qué se reconstruyó y qué se sabe que está inventado o simplificado.

## Cómo se ha hecho esta comparación

- Revisión del código de cada binario (`src/bin/*.rs`), de las banderas `REC` del launcher y de las notas de release.
- Contraste con las capturas y jars de comparación que ya están en `capturas/comparacion/` y `analisis/`.
- Para las builds cuyo jar no se conserva, la referencia es la Minecraft Wiki y el motor de `c0.0.11a`, no una copia del original.
- **No se han ejecutado las 39 builds en esta pasada ni se ha hecho una comparación fotograma a fotograma.** Donde no hay evidencia, la tabla lo dice.

## Leyenda

| Marca | Significado |
|---|---|
| **JAR** | El jar de 2009 se conserva y se usa como referencia de texturas/comportamiento. Aun así, el motor que lo ejecuta es el del port. |
| **REC** | El jar no se conserva. La build se reconstruyó desde la wiki y el motor de otra versión. |
| **DEV** | Se conserva una build de desarrollo, no necesariamente la release que lleva ese nombre. |
| **VÍDEO** | No hay jar; solo vídeo/documentación. |

## Las 39 builds

| Build | Base | Lo que hay | Diferencias conocidas |
|---|---|---|---|
| `rd-131655` | VÍDEO | Cave game: mundo 256×64×256, hierba, roca y cuevas; sin inventario, sin editar bloques y sin guardado. | No hay jar. Usa texturas de RubyDung y un generador propio; el terreno y las cuevas son una aproximación al vídeo, no el build grabado. |
| `rd-132211` | JAR | Mundo RubyDung, hierba/piedra y el velo que pulsa en el bloque apuntado. | Motor, física y render del port. Las texturas dependen de que encuentre tu jar; sin él usa el atlas de reemplazo. No es 1:1. |
| `rd-132328` | JAR | Primer mob: zombis con piel `char.png`. | El modelo y la animación son cubos propios del port, no el `ModelZombie` original. La IA y el movimiento son aproximados. |
| `rd-20090515` | JAR | Partículas, más bloques y el cubo del bloque en la esquina. | Generador, partículas y HUD son del port. La posición y el aspecto del cubo no se compararon pixel a pixel. |
| `rd-160052` | JAR | Mismo motor que `rd-20090515`, sin el tile de arbusto. | Comparte el motor del port; las diferencias reales de generación entre ambos builds no están verificadas. |
| `rd-161348` | JAR | Último pre-clásico; su jar es el mismo que el de `rd-20090515`. | Solo cambia el nombre y la carpeta de mundo. Sigue siendo el motor aproximado del port. |
| `mc-161607` | REC | Primera alpha privada: Escape libera el ratón, 1024×768, sin número de versión ni HUD. | Todo lo demás se hereda de `c0.0.11a`. La wiki solo documenta ese cambio; el resto es inventado o desconocido. |
| `mc-161616` | REC | Añade Y para invertir el eje del ratón. | Igual que la anterior: solo se simula el cambio documentado; el resto es el motor de `0.0.11a`. |
| `mc-161625` | REC | Arregla texturas y el invertido del ratón. | No se conservan las texturas de esa alpha; se usan las de `0.0.11a`. El “arreglo” es una aproximación. |
| `mc-161648` | REC | Arregla el retardo del ratón; última sin número de versión. | Misma base reconstruida; no hay evidencia para comparar el movimiento real del ratón. |
| `c0.0.3a` | REC | Cuevas, 1024×768, clic izquierdo coloca y derecho rompe, sin modo construcción ni contador de fps. | Se monta sobre `c0.0.11a`: física, render y detalles de UI posteriores pueden estar filtrados. El generador de cuevas es aproximado. |
| `c0.0.9a` | REC | Modo construcción, velo blanco por seis caras, contador de fps/chunks, 640×480 y Escape cierra. | Reconstruida; la interfaz y el modo construcción no son los del jar original, que no se conserva. |
| `c0.0.10a` | REC | Como `0.0.9a`, pero Escape solo suelta el ratón. | Solo se cambia Escape; el resto sigue siendo el motor reconstruido de `0.0.11a`. |
| `c0.0.11a` | JAR | Primera Classic conservada: `NoiseMap`, cuevas, HUD de fps/chunks y modo pintar. | Es la build más cercana, pero el render, la física y el HUD son del port. No es el jar ejecutándose. |
| `c0.0.12a` | REC | Océano, islas, lagos de lava, bedrock y natación. | Texturas base de `0.0.11a`; los tres tiles nuevos se importan de `0.0.13a_03` o se dibujan. **Si no está ese jar, la lava queda naranja plana**, no como el tile rojo/amarillo original. Generador aproximado. |
| `c0.0.12a_01` | REC | Reinicio de mobs al crear un nivel. | No hay un sistema de mobs equivalente al original; el cambio es casi cosmético. Comparte todo el motor de `0.0.12a`. |
| `c0.0.12a_02` | REC | Mejora de rendimiento, sin cambios de juego. | No se puede comparar el rendimiento con el build perdido; es el mismo `0.0.12a` del port. |
| `c0.0.12a_03` | JAR | Reupload archivado: correcciones de guardado y rendimiento. | Usa el motor compartido de `0.0.12a`, no el código del jar. Arrastra las mismas texturas de respaldo y el mismo generador aproximado. |
| `c0.0.13a` | DEV | 854×480, mundo plano 32×512×64, cuevas, lava subterránea y `level.dat` con cabecera de 0.0.13a. | Es la build de desarrollo, no la release perdida. Generador y cuevas son una reconstrucción; el guardado con cuenta no está. |
| `c0.0.13a_03` | JAR | 854×480, sin lagos de lava sobre el nivel del mar y sin mobs naturales. | Motor compartido; el agua/lava se importan del jar, pero el HUD, la física y el render son del port. |
| `c0.0.14a` | REC | Arena, grava, tres menas, árboles, bloques que caen, nubes, niebla lineal azul y paleta de nueve bloques. | Los tiles nuevos salen de `0.0.14a_08`. El generador, las vetas y los árboles son aproximaciones; la copa cuadrada se eligió por la wiki, no por un jar. |
| `c0.0.14a_08` | DEV | Copa en cruz, niebla blanca y sombras en agua/lava. | El atlas es el del jar, pero las sombras y la niebla son del render del port. No hay comparación visual completa. |
| `c0.0.15a` | REC | Grava solo bajo el agua, roble pequeño y lava más lenta/brillante. | Era un test multijugador y aquí no hay multijugador. **Las texturas de 0.0.15a no se conservan: se usa el atlas de 0.0.14a**, así que los colores no son los originales. |
| `c0.0.15a_01` | REC | Reupload con interpolación de cliente. | No hay cliente/servidor; la interpolación no se simula. Es `0.0.15a` con otro nombre. |
| `c0.0.15a_02` | REC | Reupload con animaciones corregidas. | Sin otros jugadores, el cambio queda reducido a la animación local. Texturas de respaldo de `0.0.14a`. |
| `c0.0.16a` | REC | Chat con T, `/broadcast` y `/kick`. | Los comandos de servidor se adaptaron a un jugador; no hay servidor ni otros jugadores. Es una reconstrucción, no el multijugador original. |
| `c0.0.16a_02` | REC | Niebla índigo bajo el agua. | Solo se añade la niebla; el resto es `0.0.16a` adaptado. |
| `c0.0.17a` | REC | `/setspawn`, `/spawn`, Tab y registro de chat. | La lista de jugadores muestra solo al jugador local. El comportamiento de servidor no existe. |
| `c0.0.18a` | REC | `/teleport <x> <y> <z>` y `/tp`. | El comando original era de admin sobre otros jugadores; aquí se inventó la variante por coordenadas. |
| `c0.0.18a_02` | REC | La fuente acepta `\`, `|`, `@` y `$`. | Solo se amplía la fuente/consola; el resto es `0.0.18a`. |
| `c0.0.19a` | REC | Cristal, esponja, oro, barra rápida de nueve, agua/lava animadas y límite de 100 fps. | Los tiles nuevos los dibuja el port; la animación y la esponja son aproximaciones. No son texturas de Mojang. |
| `c0.0.19a_06` | REC | Corrección del temporizador de `0.0.19a`. | Mismo contenido y mismos tiles procedurales; solo cambia el recorte de ticks. |
| `c0.0.20a` | REC | Inventario con B, dieciséis telas, flores, setas, `/solid` y bedrock irrompible. | La UI del inventario es propia; el crecimiento de plantas y la disposición no son los del original. |
| `c0.0.20a_02` | REC | Arreglo de transparencia del agua. | Solo cambia el alfa del agua; el resto es `0.0.20a`. |
| `c0.0.21a` | REC | Inventario gris, andar con él abierto, auto-clic y alcance 256. | Interfaz y alcance son del port; no se compararon con el jar original. |
| `c0.0.22a` | REC | Sonidos sintetizados al poner/romper, M silencia y lava más lenta. | Los sonidos se generan en memoria, no son los samples originales. La lava y el mezclador son aproximados. |
| `c0.0.23a` | REC | Escape abre opciones: fps, invertir ratón, distancia y sonido. | El menú es propio. La distancia y la niebla no tienen por qué coincidir con las opciones de 2009. |
| `0.24_SURVIVAL_TEST` | REC | Salud 20, flechas con Tab, creepers, cerdos, esqueletos, zombis, drops, puntuación y muerte. | La familia más simplificada: modelos, drops, IA, daño, flechas y puntuación son del port. Hay texturas y comportamientos inventados; no es una referencia. |
| `0.25_SURVIVAL_TEST` | REC | Guardado, mobs más rápidos, reaparición, peleas entre monstruos, flechas moradas, setas y puntuación siempre visible. | Todo lo anterior sigue siendo aproximado. La puntuación a la derecha y el no golpear a través de paredes se añadieron, pero el conjunto no es fiel al original. |

## Los errores que se ven en las capturas

1. **Lava plana en `c0.0.12a`.** Si el jar de `c0.0.13a_03` no está instalado, el fallback pinta un naranja con ruido, no el patrón rojo/amarillo del original. Se ve comparando `capturas/comparacion/original-0.0.12a.jpg` con `capturas/comparacion/port-0.0.12a.png`.
2. **Texturas prestadas.** `0.0.15a` usa el atlas de `0.0.14a`. Las alpha privadas y varias reconstruidas usan el atlas de `0.0.11a`. `0.0.19a` en adelante usa tiles dibujados por el port.
3. **Modelos de mobs.** Creeper, cerdo y esqueleto son geometría de cubos del port. No son los modelos ni las animaciones de 2009.
4. **Multijugador que no existe.** `0.0.15a`–`0.0.18a_02` eran tests de red; aquí están reducidos a un jugador, con comandos adaptados o inventados.
5. **UI y HUD propios.** El inventario, el menú de opciones, la barra rápida, la puntuación y el HUD se dibujan con el motor del port, no con la interfaz original.
6. **Survival Test aproximado.** La IA, el daño, los drops, las flechas y la puntuación de `0.24`/`0.25` no se compararon con el comportamiento del jar original; son la parte menos fiable.

## Qué no se ha hecho

- No se han reescrito los 39 motores ni se han recuperado jars perdidos.
- No se ha ejecutado una comparación visual de las 39 builds en esta pasada.
- No se ha verificado el movimiento, la física, la IA ni el HUD contra el original fotograma a fotograma.
- Las builds `REC` siguen siendo reconstrucciones; el aviso **BETA** del launcher es permanente hasta que se demuestre lo contrario.

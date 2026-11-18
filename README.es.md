# Rustium

Launcher y port a Rust de las versiones pre-clásicas de Minecraft (rd-132211,
rd-132328, rd-20090515, rd-160052 y rd-161348), sin cuenta de Microsoft y sin
descargar nada de Mojang: lee las texturas de la copia que ya tengas en el ordenador
y, si no hay ninguna, dibuja las suyas.

*[Read in English](README.md)*

## Cómo es

![El launcher de Rustium](docs/screenshots/launcher.png)

![Panel de rendimiento comparando Java y Rust](docs/screenshots/launcher-performance.png)

## Para jugar

Solo hace falta **el instalador**: `Rustium-0.1.5-setup.exe`.

- Nada más. No hay que instalar Rust, ni Java, ni el launcher oficial, ni una cuenta.
- Se instala por usuario en `%LOCALAPPDATA%\Rustium`, sin permisos de administrador.
- `Rustium.exe` y un ejecutable de juego por versión van dentro del instalador ya
  compilados y enlazados en estático: las únicas DLLs que usan son las del propio
  Windows.
- `.minecraft` es opcional. Java solo se usa si pulsas "Measure FPS".

## Las versiones

El selector encima de «Jugar» arranca cualquiera de las cinco versiones pre-clásicas:

| versión | qué trae |
|---|---|
| rd-132211 | el mundo de RubyDung: césped, piedra y el velo que pulsa en el bloque que miras |
| rd-132328 | el primer móvil, los zombis, con su piel de Steve |
| rd-20090515 | partículas, más bloques y el cubo del bloque en la esquina |
| rd-160052 | el mismo motor, un build anterior |
| rd-161348 | el último build pre-clásico (su jar es el mismo que el de rd-20090515) |

rd-131655 (el «Cave game tech test») nunca tuvo jar publicado, así que no hay nada que
portar de él: las cinco de arriba son las versiones pre-clásicas que Mojang publicó.

## Para compilar desde el código

Requisitos:

- Rust estable con el objetivo `x86_64-pc-windows-gnu` (`rustup target add
  x86_64-pc-windows-gnu`).
- Un **MinGW-W64 GCC** de verdad, con su `x86_64-w64-mingw32-gcc.exe`.

Las dependencias de Rust las baja cargo solo; lo que no viene solo es el enlazador.

### Por qué hace falta decir cuál usar

El `x86_64-w64-mingw32-gcc` que suele estar en el PATH es el de **llvm-mingw**, que es
clang disfrazado. Ese driver no enlaza bien el `std` de Rust para Windows: falla incluso
al compilar un build script, con

```
ld.lld: error: undefined symbol: _Unwind_SetGR
ld.lld: error: undefined symbol: _GCC_specific_handler
ld.lld: error: undefined symbol: _Unwind_RaiseException
error: could not compile `parking_lot_core` (build script)
```

Es porque el `std` de `*-windows-gnu` espera los símbolos de unwind que exporta
MinGW-W64 (`libgcc_s`/`libunwind`) y el enlazador de llvm-mingw no los pone ahí. No es
un problema del código: con un GCC de MinGW-W64 enlaza a la primera.

### Compilar

Apunta cargo al GCC bueno con una variable de entorno, sin tocar el repo:

```bash
CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER="C:/mingw64/bin/x86_64-w64-mingw32-gcc.exe" \
cargo build --release --target x86_64-pc-windows-gnu --bins
```

Cambia esa ruta por la de tu MinGW-W64 (en el PC de desarrollo está en
`C:\m\bin\x86_64-w64-mingw32-gcc.exe`). También vale un `.cargo/config.toml` local con
la misma ruta; no se versiona porque es de cada máquina.

Los ejecutables quedan en `target/x86_64-pc-windows-gnu/release/` y todos tienen que
estar en la misma carpeta, porque el launcher busca el .exe de la versión elegida al
lado.

### Instalador

Con los `.exe` ya compilados y [Inno Setup 6](https://jrsoftware.org/isinfo.php)
instalado:

```powershell
powershell -ExecutionPolicy Bypass -File installer/build.ps1
```

Escribe `dist/Rustium-0.1.5-setup.exe`, que es el archivo que se adjunta a un release.
El launcher busca ahí el asset `.exe` con su `size` y su `digest: sha256:…`, así que un
release sin SHA-256 publicado no se descarga ni se ejecuta.

## Licencia

Rustium se publica bajo la [licencia Apache 2.0](LICENSE), con el aviso de atribución
en [NOTICE](NOTICE).

Si lo redistribuyes, en código o ya compilado, tienes que conservar el aviso de
copyright y el archivo `NOTICE`, y marcar los archivos que cambies: el crédito a
**DandiMC** viaja con el código. La licencia además cede licencia de patentes y no
permite usar el nombre del proyecto como respaldo.

Eso cubre solo lo que hay aquí: no se incluye ni se licencia código, texturas ni ningún
otro material de Mojang, y el launcher no descarga nada de Mojang. Minecraft es una
marca de Mojang Studios; este proyecto no está afiliado a Mojang ni a Microsoft ni
cuenta con su respaldo.

Las tres fuentes incluidas van bajo la SIL Open Font License 1.1; sus avisos de
copyright están en [assets/fonts/OFL.txt](assets/fonts/OFL.txt).

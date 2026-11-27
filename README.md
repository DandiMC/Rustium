# Rustium

Launcher and Rust port of the pre-classic Minecraft builds (rd-131655, rd-132211,
rd-132328, rd-20090515, rd-160052 and rd-161348), the Classic private alphas
(mc-161607, mc-161616, mc-161625 and mc-161648) and the first Classics
(c0.0.3a, c0.0.9a, c0.0.10a, c0.0.11a, c0.0.12a and its continuations through
c0.0.13a_03), with no Microsoft account and nothing
downloaded from Mojang: it reads the textures of the copy already on your computer
and, if there is none, draws its own.

*[Leer en español](README.es.md)*

## How it looks

![The Rustium launcher](docs/screenshots/launcher.png)

![Classic 0.0.12a, with water and lava](docs/screenshots/classic012a.png)

![The Worlds tab](docs/screenshots/launcher-worlds.png)

![Performance panel comparing Java and Rust](docs/screenshots/launcher-performance.png)

![rd-131655, the Cave game tech test](docs/screenshots/rd-131655.png)

## To play

You only need **the installer**: `Rustium-0.3.6-setup.exe`.

- Nothing else. No Rust, no Java, no official launcher, no account.
- It installs per user into `%LOCALAPPDATA%\Rustium`, with no administrator prompt.
- `Rustium.exe` and one game executable per build ship inside the installer already
  compiled and statically linked: the only DLLs they use are the ones that come with
  Windows.
- `.minecraft` is optional. Java is only used if you press "Measure FPS".

## The versions

The picker above the Play button starts any of the thirty-four builds (the six
pre-classics, the four private alphas and the first twenty Classics):

| build | what it brings |
|---|---|
| rd-131655 | the Cave game tech test: terraced grass, rock and huge caves, with no inventory and no block editing |
| rd-132211 | the RubyDung world: grass, stone and the pulsing highlight on the block you look at |
| rd-132328 | the first mob, zombies, with their Steve skin |
| rd-20090515 | particles, more blocks and the block cube in the corner |
| rd-160052 | the same engine, one earlier build |
| rd-161348 | the last pre-classic build (its jar is the same as rd-20090515's) |
| mc-161607 | reconstructed: the first Minecraft anyone played; Escape releases the mouse, with no version number on screen |
| mc-161616 | reconstructed: Y inverts the mouse axis |
| mc-161625 | reconstructed: texture and mouse-invert fixes |
| mc-161648 | reconstructed: mouse-look fix, the last build without a version number |
| c0.0.3a | reconstructed: the caves are back, at 1024×768, before the build mode and the fps counter |
| c0.0.9a | reconstructed: build mode with the white overlay on all sides, fps counter, 640×480 and Escape quits |
| c0.0.10a | reconstructed: like 0.0.9a, but Escape only releases the mouse |
| c0.0.11a | the first Classic that survives: `NoiseMap` terrain with caves, an fps/chunk-update HUD and the paint mode the right button toggles |
| c0.0.12a | reconstructed: water, lava and bedrock: an ocean around the islands, lava lakes and swimming; only 1/2/3/4/6 select blocks, as in the archived build |
| c0.0.12a_01 | reconstructed: like 0.0.12a, with mobs resetting on a new level |
| c0.0.12a_02 | reconstructed: the performance improvement of 0.0.12a, with no gameplay changes |
| c0.0.12a_03 | the archived 0.0.12a reupload: saving and performance fixes |
| c0.0.13a | reconstructed from the archived development build: 854×480, the 0.0.11a jump, 13a water/lava physics, blue/red fog and the flat 32×512×64 world with caves and underground lava |
| c0.0.13a_03 | official and archived: 854×480, no lava lakes above sea level and no natural mobs |
| c0.0.14a | reconstructed: sand, gravel and the three ores, trees of logs and leaves, sand and gravel fall, clouds, linear blue fog, a nine-slot block palette and the 0.0.14a atlas |
| c0.0.14a_08 | development: cross-shaped canopy, white fog and shadows on water and lava |
| c0.0.15a | first multiplayer test: gravel only underwater, small oak trees and slower, brighter lava |
| c0.0.15a_01 | reupload: better client interpolation, with no gameplay changes in the port |
| c0.0.15a_02 | reupload: fixed animations, with no gameplay changes in the port |
| c0.0.16a | server commands adapted to single player: T opens the chat console, `/broadcast` writes to the log and `/kick` explains there is nobody else |
| c0.0.16a_02 | the last multiplayer test: indigo underwater fog instead of white |
| c0.0.17a | `/setspawn`, `/spawn` and the player list with Tab; the chat log appears while typing |
| c0.0.18a | `/teleport <x> <y> <z>` (or `/tp`), adapted from the admin command to coordinates |
| c0.0.18a_02 | the font accepts `\`, `|`, `@` and `$`; the chat console passes them through |
| c0.0.19a | glass, sponge, gold block, a nine-slot hotbar, animated water/lava and a 100 fps cap |
| c0.0.19a_06 | timer fix for 0.0.19a: same blocks and hotbar, steadier ticks |
| c0.0.20a | B inventory, sixteen wools, dandelions, roses, mushrooms, `/solid` and unbreakable bedrock |
| c0.0.20a_02 | water transparency fix for 0.0.20a, with the inventory and plants intact |

The four mc- builds, c0.0.3a, c0.0.9a, c0.0.10a, c0.0.12a, c0.0.12a_01, c0.0.12a_02,
c0.0.13a, c0.0.14a, c0.0.15a, c0.0.15a_01, c0.0.15a_02, c0.0.16a, c0.0.16a_02,
c0.0.17a, c0.0.18a, c0.0.18a_02, c0.0.19a and c0.0.20a are **reconstructions**: their jars were never archived. They are built
from c0.0.11a's code plus what the Minecraft Wiki documents for each, and use
c0.0.11a's textures and font when there is no jar of their own. The launcher marks
them with **REC** in the picker and the instance list; the id and world folder do not
change.

rd-131655 (the "Cave game tech test") never had a released jar. It is reconstructed
from the Minecraft Wiki and uses the same RubyDung textures as rd-132211.

`c0.0.13a` follows the jar the official launcher archives under that name: the
world-size development build, not the lost release. 0.0.14a is also lost, so its
new tiles come from the archived 0.0.14a_08 jar and its generator follows the wiki:
beaches, ore veins of 10-30 blocks and trees. The **0.0.14a** reconstruction follows
the base behaviour: square canopy and linear blue fog. `c0.0.14a_08` is a separate
build with what the wiki documents for the development version: cross-shaped top,
white fog and water/lava shadows. The `_08` jar is used only for the textures the
lost release does not keep. `c0.0.15a` moves gravel underwater, brings the small oak
canopy and makes lava flow slower and look brighter.
It generates the documented
flat 32×512×64 world, carves caves and seeds lava underground. Its `level.dat` uses
the 0.0.13a header (magic `0x271BB788`, name, creator, time and width/height/depth);
the header is also read, so levels with other dimensions such as the documented
1024×64×64 and 512×64×512 tests load and render. Saves from before 0.0.13a keep the
old raw-block format, and those old worlds still load.

## Your worlds

Each instance saves its world in `%APPDATA%\Rustium\worlds\<instance>\level.dat` by
default, and can point at any other folder from its options, so a save from
rd-132211 is never opened with another family's block ids. Default instances
reuse the build id, so worlds saved before instances existed stay where
they are. Each instance also remembers its own initial window size; leave the
resolution empty to use the size that build originally had. The launcher's **Worlds** tab shows how much space each one takes
and when you last played it, opens its folder and deletes it after asking. The
installs sit right below, each with its own folder.

## To build from source

Requirements:

- Stable Rust with the `x86_64-pc-windows-gnu` target (`rustup target add
  x86_64-pc-windows-gnu`).
- A real **MinGW-W64 GCC**, with its `x86_64-w64-mingw32-gcc.exe`.

Cargo downloads the Rust dependencies by itself; what it does not bring is the linker.

### Why you have to tell it which one to use

The `x86_64-w64-mingw32-gcc` usually sitting in PATH is the one from **llvm-mingw**,
which is clang in disguise. That driver cannot link Rust's `std` for Windows: it fails
even while building a build script, with

```
ld.lld: error: undefined symbol: _Unwind_SetGR
ld.lld: error: undefined symbol: _GCC_specific_handler
ld.lld: error: undefined symbol: _Unwind_RaiseException
error: could not compile `parking_lot_core` (build script)
```

That is because the `std` of `*-windows-gnu` expects the unwind symbols exported by
MinGW-W64 (`libgcc_s`/`libunwind`), and the llvm-mingw linker does not put them there.
It is not a problem with the code: with a MinGW-W64 GCC it links first time.

### Building

Point cargo at the good GCC with an environment variable, no repo changes needed:

```bash
CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER="C:/mingw64/bin/x86_64-w64-mingw32-gcc.exe" \
cargo build --release --target x86_64-pc-windows-gnu --bins
```

Change that path to your own MinGW-W64 (on the development PC it lives at
`C:\m\bin\x86_64-w64-mingw32-gcc.exe`). A local `.cargo/config.toml` with the same
path works too; it is not committed because it belongs to each machine.

The executables land in `target/x86_64-pc-windows-gnu/release/`, and all of them must
end up in the same folder, because the launcher looks for the exe of the selected
build next to itself.

### Installer

With the `.exe` files already built and [Inno Setup 6](https://jrsoftware.org/isinfo.php)
installed:

```powershell
powershell -ExecutionPolicy Bypass -File installer/build.ps1
```

It writes `dist/Rustium-0.3.6-setup.exe`, the file to attach to a release. The launcher
looks there for the `.exe` asset with its `size` and its `digest: sha256:…`, so a
release without a published SHA-256 is neither downloaded nor run.

## License

Rustium is released under the [Apache License 2.0](LICENSE), with the attribution
notice in [NOTICE](NOTICE).

If you redistribute it, as source or as a built program, you must keep the copyright
notice and the `NOTICE` file, and mark the files you changed: the credit to **DandiMC**
travels with the code. The license also grants a patent license and does not allow
the use of the project's name as an endorsement.

That covers only what is here: no Minecraft code, textures or other Mojang assets
are included or licensed, and the launcher downloads nothing from Mojang. Minecraft
is a trademark of Mojang Studios; this project is not affiliated with or endorsed by
Mojang or Microsoft.

The three bundled fonts are under the SIL Open Font License 1.1; their copyright
notices are in [assets/fonts/OFL.txt](assets/fonts/OFL.txt).

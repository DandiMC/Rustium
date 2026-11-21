//! Rustium: launcher for RubyDung RS.
//!
//! It does not download or redistribute Minecraft: it looks for the player's own
//! copy of rd-132211, checks that its jar carries `terrain.png`, and starts our
//! Rust game with those textures. If no copy is found the player can paste a path
//! by hand or play with the replacement texture the game draws itself.
//!
//! The only network request this program makes is an optional read of the
//! project's release list, to tell the user when a newer launcher exists.
//!
//! Usage:
//!   rustium                     detect the installed version and play
//!   rustium --status            print which textures will be used and exit
//!   rustium --atlas <path>      use a specific .jar or .png
//!   rustium --dir <folder>      look inside another .minecraft folder
//!   rustium --lang <code>       en, es, pt, fr, de
//!   rustium --console           no window, everything in the terminal
//!   rustium --help

// A window of its own, so opening it does not leave a console behind. Launched
// from a terminal, --console still writes to that terminal as before.
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

// --------------------------------------------------------------------- output
// The launcher never prints directly: everything goes through `say!`, which sends
// the line to the window when a UI is attached and to stdout otherwise.
static REPORT: Mutex<Option<Box<dyn Fn(&str) + Send + Sync>>> = Mutex::new(None);

pub fn set_report(f: Box<dyn Fn(&str) + Send + Sync>) {
    if let Ok(mut g) = REPORT.lock() {
        *g = Some(f);
    }
}

pub fn clear_report() {
    if let Ok(mut g) = REPORT.lock() {
        *g = None;
    }
}

fn emit(s: &str) {
    if let Ok(g) = REPORT.lock() {
        if let Some(f) = g.as_ref() {
            f(s);
            return;
        }
    }
    println!("{s}");
}

macro_rules! say {
    () => {
        emit("")
    };
    ($($t:tt)*) => {{
        let _s = format!($($t)*);
        emit(&_s);
    }};
}

const NAME: &str = "Rustium";
const VERSION: &str = env!("CARGO_PKG_VERSION");
/// The original version the game needs (RubyDung / Minecraft rd-132211).
const GAME_VERSION: &str = "rd-132211";
/// Where we ask whether a newer launcher exists. Override with RUSTIUM_UPDATE_URL
/// (useful for testing against a local file server).
const UPDATE_URL: &str = "https://api.github.com/repos/DandiMC/Rustium/releases/latest";
/// Where the two games are compared: the fraction of the map their spawn is pinned
/// to (the middle, above the terrain). The original gets it as -Drustium.spawn and
/// the port as RD_SPAWN, so the spot is stated once here and each side works out
/// the coordinates from the size of its own world.
const BENCH_SPAWN: f32 = 0.5;

type R<T> = Result<T, String>;

/// Las cinco versiones pre-clasicas que trae el launcher, en orden de fecha.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum GameVersion {
    Rd132211,
    Rd132328,
    Rd20090515,
    Rd160052,
    Rd161348,
}

impl GameVersion {
    const ALL: [GameVersion; 5] = [
        GameVersion::Rd132211,
        GameVersion::Rd132328,
        GameVersion::Rd20090515,
        GameVersion::Rd160052,
        GameVersion::Rd161348,
    ];

    /// The build name, as the original version is called everywhere else.
    fn id(self) -> &'static str {
        match self {
            GameVersion::Rd132211 => "rd-132211",
            GameVersion::Rd132328 => "rd-132328",
            GameVersion::Rd20090515 => "rd-20090515",
            GameVersion::Rd160052 => "rd-160052",
            GameVersion::Rd161348 => "rd-161348",
        }
    }

    /// The executable that runs this build.
    fn exe(self) -> &'static str {
        let win = cfg!(windows);
        match self {
            GameVersion::Rd132211 => {
                if win { "rubydung-rs.exe" } else { "rubydung-rs" }
            }
            GameVersion::Rd132328 => {
                if win { "rd132328.exe" } else { "rd132328" }
            }
            GameVersion::Rd20090515 => {
                if win { "rd20090515.exe" } else { "rd20090515" }
            }
            GameVersion::Rd160052 => {
                if win { "rd160052.exe" } else { "rd160052" }
            }
            GameVersion::Rd161348 => {
                if win { "rd161348.exe" } else { "rd161348" }
            }
        }
    }

    fn from_id(s: &str) -> Option<Self> {
        GameVersion::ALL.iter().copied().find(|v| v.id() == s.trim())
    }
}

#[derive(Clone)]
struct Options {
    dir_minecraft: PathBuf,
    /// Texture path chosen by hand (a .jar with terrain.png inside, or a .png).
    atlas: Option<PathBuf>,
    lang: Lang,
    /// Which pre-classic build the Play button starts.
    game: GameVersion,
    status: bool,
    help: bool,
    console: bool,
}

// ------------------------------------------------------------------ languages

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Lang {
    En,
    Es,
    Pt,
    Fr,
    De,
}

impl Lang {
    const ALL: [Lang; 5] = [Lang::En, Lang::Es, Lang::Pt, Lang::Fr, Lang::De];

    fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Es => "es",
            Lang::Pt => "pt",
            Lang::Fr => "fr",
            Lang::De => "de",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Es => "Español",
            Lang::Pt => "Português",
            Lang::Fr => "Français",
            Lang::De => "Deutsch",
        }
    }

    fn from_code(s: &str) -> Option<Lang> {
        let s = s.trim().to_lowercase();
        let head = s.split(['-', '_']).next().unwrap_or("");
        Lang::ALL.into_iter().find(|l| l.code() == head)
    }
}

/// Every string the launcher shows. `{path}`, `{version}` and `{url}` are filled
/// in with `fill()`.
#[derive(Clone, Copy)]
struct Texts {
    tagline: &'static str,
    found_title: &'static str,
    found_src: &'static str,
    manual_src: &'static str,
    missing_title: &'static str,
    missing_src: &'static str,
    missing_hint: &'static str,
    path_label: &'static str,
    use_path: &'static str,
    play: &'static str,
    refresh: &'static str,
    how: &'static str,
    launching: &'static str,
    done: &'static str,
    route_ok: &'static str,
    route_bad: &'static str,
    legal_title: &'static str,
    legal_footer: &'static str,
    how_text: &'static str,
    legal: &'static str,
    detected: &'static str,
    manual: &'static str,
    tex_found: &'static str,
    tex_none: &'static str,
    tex_replacement: &'static str,
    starting: &'static str,
    game_exited: &'static str,
    no_game: &'static str,
    help: &'static str,
}

fn fill(template: &str, key: &str, value: &str) -> String {
    template.replace(key, value)
}

fn texts(l: Lang) -> Texts {
    match l {
        Lang::En => Texts {
            tagline: "rd-132211 in Rust",
            found_title: "ORIGINAL TEXTURES",
            found_src: "found on this computer",
            manual_src: "loaded from the path you chose",
            missing_title: "REPLACEMENT TEXTURE",
            missing_src: "no rd-132211 copy on this computer",
            missing_hint: "You can still play. To see the original textures, install \
                           rd-132211 in your launcher or paste the path below.",
            path_label: "ASSET PATH  (.jar with terrain.png inside, or .png)",
            use_path: "Use this path",
            play: "Play",
            refresh: "Refresh",
            how: "How to install",
            launching: "Starting the game...",
            done: "Done. You can play again.",
            route_ok: "Path accepted: {path}",
            route_bad: "That path is not usable (not a jar with terrain.png, not a png): {path}",
            legal_title: "Legal notice",
            legal_footer: "Not affiliated with Microsoft or Mojang · does not include or download Minecraft content",
            how_text: "To get the original textures:\n\n\
                       1. Open the official Minecraft launcher (or MultiMC / Prism / ATLauncher) and \
                       install rd-132211 (enable old versions to see it: pre-classic).\n\
                       2. Come back here and press Refresh.\n\n\
                       If it lives somewhere else, paste the full path to rd-132211.jar (or to a \
                       terrain.png) above and press Use this path.",
            legal: "Rustium is not affiliated with, endorsed by or authorised by Microsoft or Mojang. \
                    \"Minecraft\" is a trademark of Mojang AB.\n\n\
                    This program does not include or download Minecraft content. It opens the copy you \
                    already have on this computer only to read its textures, and if it cannot find one \
                    it draws its own replacement texture.\n\n\
                    Your Minecraft licence applies to your own copy; this launcher does not modify or \
                    redistribute it. The only network request is an optional version check against the \
                    project's release list, which sends no information about you.\n\n\
                    Rustium is Copyright (c) 2026 DandiMC and is distributed under the Apache License \
                    2.0; the LICENSE and NOTICE files that come with the program must travel with any \
                    copy of it.",
            detected: "detected",
            manual: "chosen by hand",
            tex_found: "  Textures: {path} ({source})",
            tex_none: "  No Minecraft {version} installation found on this computer.",
            tex_replacement: "  Using the replacement texture that ships with the game.",
            starting: "  Starting {path}",
            game_exited: "  The game exited ({status})",
            no_game: "cannot find {name}. Build it with:\n  cargo +stable-x86_64-pc-windows-gnu build --release --target x86_64-pc-windows-gnu",
            help: "{name} {version} - launcher for RubyDung RS\n\n\
                   Rustium does not download or redistribute Minecraft. It reads the textures of the\n\
                   copy you already have installed; if it cannot find one it starts the game with its\n\
                   own replacement texture.\n\n\
                   Usage: {name} [options]\n\n\
                   Options:\n  \
                   --status          print which textures will be used and exit\n  \
                   --atlas <path>    use that .jar (with terrain.png inside) or .png\n  \
                   --dir <folder>    look in another .minecraft folder\n  \
                   --lang <code>     en, es, pt, fr, de\n  \
                   --console         no window, everything in the terminal\n  \
                   --help            this help",
        },
        Lang::Es => Texts {
            tagline: "rd-132211 en Rust",
            found_title: "TEXTURAS ORIGINALES",
            found_src: "encontrada en tu equipo",
            manual_src: "cargada desde la ruta que indicaste",
            missing_title: "TEXTURA DE REEMPLAZO",
            missing_src: "no hay copia de rd-132211 en este equipo",
            missing_hint: "Puedes jugar igualmente. Para ver las texturas del original, instala \
                           rd-132211 en tu launcher o indica la ruta aquí abajo.",
            path_label: "RUTA DEL ASSET  (.jar con terrain.png dentro, o .png)",
            use_path: "Usar esta ruta",
            play: "Jugar",
            refresh: "Revisar",
            how: "Cómo instalarlo",
            launching: "Arrancando el juego...",
            done: "Listo. Puedes volver a jugar.",
            route_ok: "Ruta aceptada: {path}",
            route_bad: "Esa ruta no sirve (no es un jar con terrain.png ni un png): {path}",
            legal_title: "Aviso legal",
            legal_footer: "No afiliado a Microsoft ni a Mojang · no incluye ni descarga contenido de Minecraft",
            how_text: "Para tener las texturas originales:\n\n\
                       1. Abre el launcher oficial de Minecraft (o MultiMC / Prism / ATLauncher) e \
                       instala la versión rd-132211 (aparece al activar las versiones antiguas: pre-classic).\n\
                       2. Vuelve aquí y pulsa Revisar.\n\n\
                       Si la tienes en otra carpeta, pega arriba la ruta completa del rd-132211.jar (o de \
                       un terrain.png) y pulsa Usar esta ruta.",
            legal: "Rustium no está afiliado, asociado, autorizado ni respaldado por Microsoft ni por Mojang. \
                    \"Minecraft\" es una marca de Mojang AB.\n\n\
                    Este programa no incluye ni descarga contenido de Minecraft. Abre la copia que ya tienes \
                    en este equipo solo para leer sus texturas y, si no encuentra ninguna, dibuja su propia \
                    textura de reemplazo.\n\n\
                    La licencia de Minecraft se aplica a tu copia; este launcher no la modifica ni la \
                    redistribuye. La única petición de red es una comprobación opcional de versión contra la \
                    lista de versiones del proyecto, que no envía ninguna información sobre ti.\n\n\
                    Rustium es Copyright (c) 2026 DandiMC y se distribuye bajo la licencia Apache 2.0; \
                    los archivos LICENSE y NOTICE que acompañan al programa deben conservarse en \
                    cualquier copia.",
            detected: "detectada",
            manual: "ruta elegida a mano",
            tex_found: "  Texturas: {path} ({source})",
            tex_none: "  No encuentro Minecraft {version} instalado en este equipo.",
            tex_replacement: "  Se usará la textura de reemplazo que trae el juego.",
            starting: "  Arrancando {path}",
            game_exited: "  El juego terminó ({status})",
            no_game: "no encuentro {name}. Compílalo con:\n  cargo +stable-x86_64-pc-windows-gnu build --release --target x86_64-pc-windows-gnu",
            help: "{name} {version} - launcher de RubyDung RS\n\n\
                   Rustium no descarga ni redistribuye Minecraft. Lee las texturas de la copia que ya\n\
                   tengas instalada; si no encuentra ninguna, arranca el juego con su propia textura de\n\
                   reemplazo.\n\n\
                   Uso: {name} [opciones]\n\n\
                   Opciones:\n  \
                   --status          dice qué texturas se van a usar y sale\n  \
                   --atlas <ruta>    usa ese .jar (con terrain.png dentro) o .png\n  \
                   --dir <carpeta>   mira en otra carpeta .minecraft\n  \
                   --lang <code>     en, es, pt, fr, de\n  \
                   --console         sin ventana, todo en la terminal\n  \
                   --help            esta ayuda",
        },
        Lang::Pt => Texts {
            tagline: "rd-132211 em Rust",
            found_title: "TEXTURAS ORIGINAIS",
            found_src: "encontradas neste computador",
            manual_src: "carregadas do caminho que indicaste",
            missing_title: "TEXTURA SUBSTITUTA",
            missing_src: "não há cópia de rd-132211 neste computador",
            missing_hint: "Podes jogar na mesma. Para ver as texturas originais, instala \
                           rd-132211 no teu launcher ou indica o caminho aqui em baixo.",
            path_label: "CAMINHO DO ASSET  (.jar com terrain.png dentro, ou .png)",
            use_path: "Usar este caminho",
            play: "Jogar",
            refresh: "Procurar",
            how: "Como instalar",
            launching: "A iniciar o jogo...",
            done: "Pronto. Podes voltar a jogar.",
            route_ok: "Caminho aceite: {path}",
            route_bad: "Esse caminho não serve (não é um jar com terrain.png nem um png): {path}",
            legal_title: "Aviso legal",
            legal_footer: "Sem vínculo com a Microsoft ou a Mojang · não inclui nem descarrega conteúdo do Minecraft",
            how_text: "Para teres as texturas originais:\n\n\
                       1. Abre o launcher oficial do Minecraft (ou MultiMC / Prism / ATLauncher) e \
                       instala a versão rd-132211 (aparece ao ativar as versões antigas: pre-classic).\n\
                       2. Volta aqui e carrega em Procurar.\n\n\
                       Se estiver noutra pasta, cola acima o caminho completo do rd-132211.jar (ou de um \
                       terrain.png) e carrega em Usar este caminho.",
            legal: "O Rustium não está afiliado, associado, autorizado nem apoiado pela Microsoft nem pela \
                    Mojang. \"Minecraft\" é uma marca da Mojang AB.\n\n\
                    Este programa não inclui nem descarrega conteúdo do Minecraft. Abre a cópia que já tens \
                    neste computador apenas para ler as suas texturas e, se não encontrar nenhuma, desenha a \
                    sua própria textura substituta.\n\n\
                    A licença do Minecraft aplica-se à tua cópia; este launcher não a modifica nem a \
                    redistribui. O único pedido de rede é uma verificação opcional de versão contra a lista \
                    de versões do projeto, que não envia informação sobre ti.\n\n\
                    O Rustium é Copyright (c) 2026 DandiMC e é distribuído ao abrigo da licença \
                    Apache 2.0; os ficheiros LICENSE e NOTICE que acompanham o programa devem ser \
                    mantidos em qualquer cópia.",
            detected: "detetada",
            manual: "caminho escolhido à mão",
            tex_found: "  Texturas: {path} ({source})",
            tex_none: "  Não encontro o Minecraft {version} instalado neste computador.",
            tex_replacement: "  Vai ser usada a textura substituta que vem com o jogo.",
            starting: "  A arrancar {path}",
            game_exited: "  O jogo terminou ({status})",
            no_game: "não encontro {name}. Compila-o com:\n  cargo +stable-x86_64-pc-windows-gnu build --release --target x86_64-pc-windows-gnu",
            help: "{name} {version} - launcher do RubyDung RS\n\n\
                   O Rustium não descarrega nem redistribui o Minecraft. Lê as texturas da cópia que já\n\
                   tens instalada; se não encontrar nenhuma, arranca o jogo com a sua própria textura\n\
                   substituta.\n\n\
                   Uso: {name} [opções]\n\n\
                   Opções:\n  \
                   --status          diz que texturas vão ser usadas e sai\n  \
                   --atlas <caminho> usa esse .jar (com terrain.png dentro) ou .png\n  \
                   --dir <pasta>     procura noutra pasta .minecraft\n  \
                   --lang <code>     en, es, pt, fr, de\n  \
                   --console         sem janela, tudo no terminal\n  \
                   --help            esta ajuda",
        },
        Lang::Fr => Texts {
            tagline: "rd-132211 en Rust",
            found_title: "TEXTURES D'ORIGINE",
            found_src: "trouvées sur cet ordinateur",
            manual_src: "chargées depuis le chemin indiqué",
            missing_title: "TEXTURE DE REMPLACEMENT",
            missing_src: "aucune copie de rd-132211 sur cet ordinateur",
            missing_hint: "Vous pouvez jouer quand même. Pour voir les textures d'origine, installez \
                           rd-132211 dans votre launcher ou indiquez le chemin ci-dessous.",
            path_label: "CHEMIN DE L'ASSET  (.jar contenant terrain.png, ou .png)",
            use_path: "Utiliser ce chemin",
            play: "Jouer",
            refresh: "Actualiser",
            how: "Comment l'installer",
            launching: "Démarrage du jeu...",
            done: "Terminé. Vous pouvez rejouer.",
            route_ok: "Chemin accepté : {path}",
            route_bad: "Ce chemin ne convient pas (ni jar avec terrain.png, ni png) : {path}",
            legal_title: "Mentions légales",
            legal_footer: "Sans lien avec Microsoft ou Mojang · n'inclut pas et ne télécharge pas de contenu Minecraft",
            how_text: "Pour avoir les textures d'origine :\n\n\
                       1. Ouvrez le launcher officiel de Minecraft (ou MultiMC / Prism / ATLauncher) et \
                       installez la version rd-132211 (activez les anciennes versions : pre-classic).\n\
                       2. Revenez ici et cliquez sur Actualiser.\n\n\
                       Si elle se trouve ailleurs, collez ci-dessus le chemin complet du rd-132211.jar (ou \
                       d'un terrain.png) et cliquez sur Utiliser ce chemin.",
            legal: "Rustium n'est ni affilié à, ni approuvé par, ni autorisé par Microsoft ou Mojang. \
                    « Minecraft » est une marque de Mojang AB.\n\n\
                    Ce programme n'inclut ni ne télécharge de contenu Minecraft. Il ouvre la copie que vous \
                    avez déjà sur cet ordinateur uniquement pour lire ses textures et, s'il n'en trouve \
                    aucune, dessine sa propre texture de remplacement.\n\n\
                    Votre licence Minecraft s'applique à votre propre copie ; ce launcher ne la modifie pas \
                    et ne la redistribue pas. La seule requête réseau est une vérification facultative de \
                    version auprès de la liste des versions du projet, qui n'envoie rien sur vous.\n\n\
                    Rustium est Copyright (c) 2026 DandiMC et est distribué sous la licence Apache \
                    2.0 ; les fichiers LICENSE et NOTICE fournis avec le programme doivent être \
                    conservés avec toute copie.",
            detected: "détectée",
            manual: "chemin choisi à la main",
            tex_found: "  Textures : {path} ({source})",
            tex_none: "  Aucune installation de Minecraft {version} trouvée sur cet ordinateur.",
            tex_replacement: "  Utilisation de la texture de remplacement fournie avec le jeu.",
            starting: "  Démarrage de {path}",
            game_exited: "  Le jeu s'est fermé ({status})",
            no_game: "impossible de trouver {name}. Compilez-le avec :\n  cargo +stable-x86_64-pc-windows-gnu build --release --target x86_64-pc-windows-gnu",
            help: "{name} {version} - launcher pour RubyDung RS\n\n\
                   Rustium ne télécharge ni ne redistribue Minecraft. Il lit les textures de la copie que\n\
                   vous avez déjà installée ; s'il n'en trouve aucune, il lance le jeu avec sa propre\n\
                   texture de remplacement.\n\n\
                   Utilisation : {name} [options]\n\n\
                   Options :\n  \
                   --status          affiche les textures utilisées et quitte\n  \
                   --atlas <chemin>  utilise ce .jar (avec terrain.png) ou .png\n  \
                   --dir <dossier>   cherche dans un autre dossier .minecraft\n  \
                   --lang <code>     en, es, pt, fr, de\n  \
                   --console         sans fenêtre, tout dans le terminal\n  \
                   --help            cette aide",
        },
        Lang::De => Texts {
            tagline: "rd-132211 in Rust",
            found_title: "ORIGINALTEXTUREN",
            found_src: "auf diesem Computer gefunden",
            manual_src: "aus dem angegebenen Pfad geladen",
            missing_title: "ERSATZTEXTUR",
            missing_src: "keine rd-132211-Kopie auf diesem Computer",
            missing_hint: "Du kannst trotzdem spielen. Für die Originaltexturen installiere \
                           rd-132211 in deinem Launcher oder gib unten den Pfad an.",
            path_label: "PFAD ZUM ASSET  (.jar mit terrain.png darin, oder .png)",
            use_path: "Diesen Pfad verwenden",
            play: "Spielen",
            refresh: "Suchen",
            how: "Installation",
            launching: "Spiel wird gestartet...",
            done: "Fertig. Du kannst wieder spielen.",
            route_ok: "Pfad übernommen: {path}",
            route_bad: "Dieser Pfad taugt nicht (kein jar mit terrain.png, kein png): {path}",
            legal_title: "Rechtlicher Hinweis",
            legal_footer: "Nicht mit Microsoft oder Mojang verbunden · enthält und lädt keine Minecraft-Inhalte",
            how_text: "So bekommst du die Originaltexturen:\n\n\
                       1. Öffne den offiziellen Minecraft-Launcher (oder MultiMC / Prism / ATLauncher) und \
                       installiere rd-132211 (alte Versionen aktivieren: pre-classic).\n\
                       2. Komm hierher zurück und klicke auf Suchen.\n\n\
                       Liegt sie woanders, füge oben den kompletten Pfad zur rd-132211.jar (oder zu einer \
                       terrain.png) ein und klicke auf Diesen Pfad verwenden.",
            legal: "Rustium ist weder mit Microsoft noch mit Mojang verbunden, von ihnen autorisiert oder \
                    unterstützt. „Minecraft“ ist eine Marke von Mojang AB.\n\n\
                    Dieses Programm enthält keine Minecraft-Inhalte und lädt keine herunter. Es öffnet die \
                    Kopie, die du bereits auf diesem Computer hast, nur um deren Texturen zu lesen; findet es \
                    keine, zeichnet es eine eigene Ersatztextur.\n\n\
                    Deine Minecraft-Lizenz gilt für deine eigene Kopie; dieser Launcher verändert sie nicht \
                    und gibt sie nicht weiter. Die einzige Netzwerkanfrage ist eine optionale \
                    Versionsprüfung gegen die Versionsliste des Projekts, die nichts über dich sendet.\n\n\
                    Rustium ist Copyright (c) 2026 DandiMC und wird unter der Apache-Lizenz 2.0 \
                    verbreitet; die Dateien LICENSE und NOTICE neben dem Programm müssen bei jeder \
                    Kopie erhalten bleiben.",
            detected: "gefunden",
            manual: "Pfad von Hand gewählt",
            tex_found: "  Texturen: {path} ({source})",
            tex_none: "  Keine Minecraft-{version}-Installation auf diesem Computer gefunden.",
            tex_replacement: "  Es wird die mitgelieferte Ersatztextur verwendet.",
            starting: "  Starte {path}",
            game_exited: "  Das Spiel wurde beendet ({status})",
            no_game: "{name} nicht gefunden. Baue es mit:\n  cargo +stable-x86_64-pc-windows-gnu build --release --target x86_64-pc-windows-gnu",
            help: "{name} {version} - Launcher für RubyDung RS\n\n\
                   Rustium lädt Minecraft weder herunter noch gibt es Minecraft weiter. Es liest die\n\
                   Texturen der bereits installierten Kopie; findet es keine, startet es das Spiel mit\n\
                   seiner eigenen Ersatztextur.\n\n\
                   Aufruf: {name} [Optionen]\n\n\
                   Optionen:\n  \
                   --status          zeigt die verwendeten Texturen und beendet\n  \
                   --atlas <pfad>    nutzt diese .jar (mit terrain.png) oder .png\n  \
                   --dir <ordner>    sucht in einem anderen .minecraft-Ordner\n  \
                   --lang <code>     en, es, pt, fr, de\n  \
                   --console         kein Fenster, alles im Terminal\n  \
                   --help            diese Hilfe",
        },
    }
}

/// Strings for the launcher shell (rail, tabs, news, update banner). Kept apart
/// from `Texts` so the launcher layout can change without touching the console
/// messages.
#[derive(Clone, Copy)]
struct UiTexts {
    sidebar_instances: &'static str,
    instance_ready: &'static str,
    instance_replacement: &'static str,
    version_label: &'static str,
    language_label: &'static str,
    news_shortcut: &'static str,
    folder: &'static str,
    tab_home: &'static str,
    tab_news: &'static str,
    news_title: &'static str,
    n1_title: &'static str,
    n1_date: &'static str,
    n1_short: &'static str,
    n1_body: &'static str,
    n2_short: &'static str,
    n2_title: &'static str,
    n2_date: &'static str,
    n2_body: &'static str,
    update_available: &'static str,
    update_now: &'static str,
    update_downloading: &'static str,
    update_ready: &'static str,
    update_page: &'static str,
    // Why an update stopped before running anything. `{path}`, `{err}`, `{size}`,
    // `{limit}`, `{expected}` and `{got}` are filled in with `fill()`.
    upd_no_installer: &'static str,
    upd_no_folder: &'static str,
    upd_cannot_create: &'static str,
    upd_https_only: &'static str,
    upd_no_digest: &'static str,
    upd_too_big: &'static str,
    upd_download_failed: &'static str,
    upd_cannot_write: &'static str,
    upd_interrupted: &'static str,
    upd_over_cap: &'static str,
    upd_sha_mismatch: &'static str,
    see_more: &'static str,
    see_less: &'static str,
    measure_fps: &'static str,
    bench_running: &'static str,
    bench_java_wait: &'static str,
}

/// Small labels for the stat cards and the news footer. Kept as a tuple so adding
/// a language stays a one-liner.
fn extra_texts(l: Lang) -> (&'static str, &'static str, &'static str, &'static str) {
    // (game version, textures, performance, "more coming")
    match l {
        Lang::En => ("GAME VERSION", "TEXTURES", "PERFORMANCE", "More news coming soon."),
        Lang::Es => ("VERSIÓN DEL JUEGO", "TEXTURAS", "RENDIMIENTO", "Más novedades pronto."),
        Lang::Pt => ("VERSÃO DO JOGO", "TEXTURAS", "DESEMPENHO", "Mais novidades em breve."),
        Lang::Fr => ("VERSION DU JEU", "TEXTURES", "PERFORMANCES", "D'autres nouvelles bientôt."),
        Lang::De => ("SPIELVERSION", "TEXTUREN", "LEISTUNG", "Mehr Neuigkeiten folgen."),
    }
}


/// Novedades de las versiones del launcher (0.1.4 a 0.1.13).
/// (titulo, fecha, resumen, cuerpo)
fn version_news(l: Lang) -> [(&'static str, &'static str, &'static str, &'static str); 7] {
    match l {
        Lang::En => [
            (
                "The first five versions",
                "18 Nov 2026",
                "All five early Minecraft versions are now one click away.",
                "Pick any of them from the list and press Play. Each one opens in its own window, and the launcher remembers the last version you used. rd-131655 is not included because Mojang never published its file.",
            ),
            (
                "The bushes look right again?",
                "18 Nov 2026",
                "The bushes of 15 May 2009 have their leaves and crossed shape back.",
                "They used to look like a square green block. Now they are drawn with two crossed planes and their leaves, just like the original game, and you can place one with the 6 key.",
            ),
            (
                "The characters dance again",
                "19 Nov 2026",
                "Zombies move their arms, legs and head like in the original Minecraft.",
                "They used to look stiff because their movement was almost invisible. Now they walk and dance as smoothly as the original, without jerking.",
            ),
            (
                "The bushes stand up again",
                "20 Nov 2026",
                "In rd-20090515 and rd-161348 the bushes are upright again.",
                "Their crossed planes were built with the depth axis as height, so they lay on their side. They now grow from the ground to the top of the block, exactly like the original.",
            ),
            (
                "The world ticks at 20 Hz",
                "20 Nov 2026",
                "The pre-classic builds now move at the speed of the original Java game.",
                "The legacy engine ran at 60 ticks per second, so the player, zombies and particles moved three times too fast. The world now ticks at 20 Hz, physics uses the original constants, and the camera interpolates between ticks.",
            ),
            (
                "Grass and bushes update live",
                "20 Nov 2026",
                "Grass that grows and bushes that break update the world immediately.",
                "When the level changed a block on its own, that part of the world kept showing the old block until the next click. Every change now marks its area and the mesh is rebuilt in the same frame. Bushes also no longer stop the player or block light, just like the original.",
            ),
            (
                "Zombies can't crash the game anymore!",
                "21 Nov 2026",
                "All five versions open even with a zombie right in front.",
                "In the versions with zombies, the game closed as soon as one appeared on screen. It's fixed: press Play, find a zombie and watch it stay by your side. All five versions now run from start to finish.",
            ),
        ],
        Lang::Es => [
            (
                "Las cinco primeras versiones",
                "18 nov 2026",
                "Ya puedes jugar las cinco primeras versiones de Minecraft desde el mismo launcher.",
                "Elige cualquiera en la lista y pulsa Jugar. Cada una se abre en su propia ventana y el launcher recuerda la última que usaste. La rd-131655 no está porque Mojang nunca publicó su archivo.",
            ),
            (
                "¿Los arbustos vuelven a verse bien?",
                "18 nov 2026",
                "En la versión del 15 de mayo de 2009, los arbustos recuperan sus hojas y su forma cruzada.",
                "Antes parecían un bloque verde cuadrado. Ahora se dibujan con dos planos cruzados y sus hojas, igual que en el juego original, y puedes colocar uno con la tecla 6.",
            ),
            (
                "Los personajes ya bailan",
                "19 nov 2026",
                "Los zombis mueven los brazos, las piernas y la cabeza como en el Minecraft original.",
                "Antes parecían rígidos porque su movimiento casi no se notaba. Ahora caminan y bailan con la misma soltura que el original, sin dar tirones.",
            ),
            (
                "Los arbustos vuelven a crecer de pie",
                "20 nov 2026",
                "En rd-20090515 y rd-161348 los arbustos ya se ven verticales.",
                "Sus dos planos cruzados usaban la profundidad como altura y quedaban tumbados. Ahora van del suelo al techo del bloque, igual que en el original.",
            ),
            (
                "El mundo late a 20 Hz",
                "20 nov 2026",
                "Las versiones preclásicas se mueven al ritmo del Java original.",
                "El motor legacy corría a 60 ticks por segundo, así que el jugador, los zombis y las partículas iban tres veces más rápido. Ahora el mundo late a 20 Hz, la física usa las constantes del original y la cámara interpola entre ticks.",
            ),
            (
                "La hierba cambia al instante",
                "20 nov 2026",
                "La hierba que crece y los arbustos que se rompen actualizan el mundo al instante.",
                "Cuando el nivel cambiaba un bloque por su cuenta, el trozo de mundo seguía enseñando el bloque viejo hasta el siguiente clic. Ahora cada cambio marca su zona y la malla se reconstruye en el mismo fotograma. Además, los arbustos ya no frenan al jugador ni bloquean la luz, como en el original.",
            ),
            (
                "¡Los zombis ya no se comen el juego!",
                "21 nov 2026",
                "Las cinco versiones se abren aunque tengas un zombi delante.",
                "En las versiones con zombis, el juego se cerraba en cuanto uno aparecía en pantalla. Ya está arreglado: dale a Jugar, busca un zombi y comprueba que ahora se queda a tu lado. Las cinco versiones funcionan de principio a fin.",
            ),
        ],
        Lang::Pt => [
            (
                "As cinco primeiras versões",
                "18 nov 2026",
                "Já podes jogar as cinco primeiras versões de Minecraft a partir do mesmo launcher.",
                "Escolhe qualquer uma na lista e carrega em Jogar. Cada uma abre na sua própria janela e o launcher lembra a última que usaste. A rd-131655 não está porque a Mojang nunca publicou o ficheiro.",
            ),
            (
                "Os arbustos voltam a parecer certos?",
                "18 nov 2026",
                "Na versão de 15 de maio de 2009, os arbustos voltam a ter as folhas e a forma cruzada.",
                "Antes pareciam um bloco verde quadrado. Agora são desenhados com dois planos cruzados e as suas folhas, tal como no jogo original, e podes colocar um com a tecla 6.",
            ),
            (
                "Os personagens voltaram a dançar",
                "19 nov 2026",
                "Os zombis mexem os braços, as pernas e a cabeça como no Minecraft original.",
                "Antes pareciam rígidos porque o movimento quase não se notava. Agora andam e dançam com a mesma naturalidade do original, sem solavancos.",
            ),
            (
                "Os arbustos voltam a crescer de pé",
                "20 nov 2026",
                "Em rd-20090515 e rd-161348 os arbustos já aparecem verticais.",
                "Os dois planos cruzados usavam a profundidade como altura e ficavam deitados. Agora vão do chão ao topo do bloco, tal como no original.",
            ),
            (
                "O mundo corre a 20 Hz",
                "20 nov 2026",
                "As versões pré-clássicas voltam a mover-se ao ritmo do Java original.",
                "O motor legacy corria a 60 ticks por segundo, por isso o jogador, os zombis e as partículas andavam três vezes mais depressa. Agora o mundo bate a 20 Hz, a física usa as constantes do original e a câmara interpola entre ticks.",
            ),
            (
                "A relva muda ao instante",
                "20 nov 2026",
                "A relva que cresce e os arbustos que se quebram atualizam o mundo na hora.",
                "Quando o nível mudava um bloco sozinho, aquela parte do mundo continuava a mostrar o bloco antigo até ao clique seguinte. Agora cada mudança marca a sua zona e a malha é reconstruída no mesmo fotograma. Além disso, os arbustos já não travam o jogador nem bloqueiam a luz, como no original.",
            ),
            (
                "Os zombis já não fecham o jogo!",
                "21 nov 2026",
                "As cinco versões abrem mesmo com um zombi à frente.",
                "Nas versões com zombis, o jogo fechava assim que um aparecia no ecrã. Já está corrigido: carrega em Jogar, encontra um zombi e vê-o ficar ao teu lado. As cinco versões funcionam do início ao fim.",
            ),
        ],
        Lang::Fr => [
            (
                "Les cinq premières versions",
                "18 nov. 2026",
                "Tu peux maintenant jouer aux cinq premières versions de Minecraft depuis le même launcher.",
                "Choisis-en une dans la liste et appuie sur Jouer. Chacune s'ouvre dans sa propre fenêtre et le launcher retient la dernière utilisée. rd-131655 n'est pas incluse car Mojang n'a jamais publié son fichier.",
            ),
            (
                "Les buissons ont retrouvé leur aspect ?",
                "18 nov. 2026",
                "Dans la version du 15 mai 2009, les buissons ont retrouvé leurs feuilles et leur forme croisée.",
                "Avant, ils ressemblaient à un bloc vert carré. Maintenant, ils sont dessinés avec deux plans croisés et leurs feuilles, comme dans le jeu original, et tu peux en poser un avec la touche 6.",
            ),
            (
                "Les personnages dansent à nouveau",
                "19 nov. 2026",
                "Les zombies bougent les bras, les jambes et la tête comme dans le Minecraft original.",
                "Avant, ils semblaient raides car leur mouvement était presque invisible. Maintenant, ils marchent et dansent aussi naturellement que dans l'original, sans à-coups.",
            ),
            (
                "Les buissons se tiennent à nouveau debout",
                "20 nov. 2026",
                "Dans rd-20090515 et rd-161348, les buissons sont de nouveau verticaux.",
                "Leurs deux plans croisés utilisaient la profondeur comme hauteur et restaient couchés. Ils vont maintenant du sol au haut du bloc, comme dans l'original.",
            ),
            (
                "Le monde tourne à 20 Hz",
                "20 nov. 2026",
                "Les versions pré-classiques bougent au rythme du jeu Java original.",
                "Le moteur legacy tournait à 60 ticks par seconde : le joueur, les zombies et les particules allaient trois fois trop vite. Le monde bat maintenant à 20 Hz, la physique utilise les constantes d'origine et la caméra interpole entre les ticks.",
            ),
            (
                "L'herbe change à l'écran",
                "20 nov. 2026",
                "L'herbe qui pousse et les buissons qui se cassent mettent le monde à jour immédiatement.",
                "Quand le niveau changeait un bloc tout seul, cette partie du monde continuait d'afficher l'ancien bloc jusqu'au clic suivant. Chaque changement marque maintenant sa zone et le maillage est reconstruit dans la même image. Les buissons ne bloquent plus le joueur ni la lumière, comme dans l'original.",
            ),
            (
                "Les zombies ne font plus planter le jeu !",
                "21 nov. 2026",
                "Les cinq versions s'ouvrent même avec un zombie devant vous.",
                "Dans les versions avec des zombies, le jeu se fermait dès qu'un zombie apparaissait à l'écran. C'est corrigé : appuyez sur Jouer, trouvez un zombie et voyez-le rester à vos côtés. Les cinq versions fonctionnent du début à la fin.",
            ),
        ],
        Lang::De => [
            (
                "Die ersten fünf Versionen",
                "18. Nov. 2026",
                "Du kannst jetzt alle fünf frühen Minecraft-Versionen aus einem Launcher spielen.",
                "Wähle eine aus der Liste und drücke Spielen. Jede öffnet sich in einem eigenen Fenster und der Launcher merkt sich die letzte. rd-131655 fehlt, weil Mojang ihre Datei nie veröffentlicht hat.",
            ),
            (
                "Die Büsche sehen wieder richtig aus?",
                "18. Nov. 2026",
                "In der Version vom 15. Mai 2009 haben die Büsche wieder ihre Blätter und ihre gekreuzte Form.",
                "Früher sahen sie wie ein grüner quadratischer Block aus. Jetzt werden sie mit zwei gekreuzten Flächen und ihren Blättern gezeichnet, wie im Original, und du kannst mit der Taste 6 einen setzen.",
            ),
            (
                "Die Figuren tanzen wieder",
                "19. Nov. 2026",
                "Zombies bewegen Arme, Beine und Kopf wie im originalen Minecraft.",
                "Früher wirkten sie steif, weil ihre Bewegung fast unsichtbar war. Jetzt laufen und tanzen sie so locker wie im Original, ohne Ruckeln.",
            ),
            (
                "Die Büsche stehen wieder aufrecht",
                "20. Nov. 2026",
                "In rd-20090515 und rd-161348 stehen die Büsche wieder aufrecht.",
                "Ihre zwei gekreuzten Flächen nutzten die Tiefe als Höhe und lagen deshalb flach. Jetzt reichen sie vom Boden bis zur Blockoberseite, genau wie im Original.",
            ),
            (
                "Die Welt läuft mit 20 Hz",
                "20. Nov. 2026",
                "Die pre-classic-Versionen bewegen sich im Tempo des Java-Originals.",
                "Die Legacy-Engine lief mit 60 Ticks pro Sekunde, also waren Spieler, Zombies und Partikel dreimal zu schnell. Jetzt tickt die Welt mit 20 Hz, die Physik nutzt die Originalkonstanten und die Kamera interpoliert zwischen den Ticks.",
            ),
            (
                "Gras ändert sich sofort",
                "20. Nov. 2026",
                "Wachsendes Gras und zerbrechende Büsche aktualisieren die Welt sofort.",
                "Wenn das Level einen Block von selbst änderte, zeigte dieser Teil der Welt bis zum nächsten Klick noch den alten Block. Jede Änderung markiert jetzt ihre Zone und das Mesh wird im selben Frame neu gebaut. Büsche bremsen den Spieler außerdem nicht mehr und blockieren kein Licht, wie im Original.",
            ),
            (
                "Zombies bringen das Spiel nicht mehr zum Absturz!",
                "21. Nov. 2026",
                "Alle fünf Versionen starten, auch mit einem Zombie direkt vor dir.",
                "In den Versionen mit Zombies schloss sich das Spiel, sobald einer auf dem Bildschirm erschien. Jetzt behoben: Drücke Spielen, suche einen Zombie und sieh zu, wie er an deiner Seite bleibt. Alle fünf Versionen laufen von Anfang bis Ende.",
            ),
        ],
    }
}

/// Extra cards from `news.txt` next to the launcher (or in `%APPDATA%\Rustium`).
/// The developer launcher writes that file, so news can be added without a
/// rebuild. Each block is a short key=value list, blocks separated by `---`:
///
/// ```text
/// lang=es
/// title=El arbusto ya se ve de pie
/// date=20 nov 2026
/// summary=...
/// body=...
/// art=1
/// ```
///
/// `version` is optional (e.g. `rd-20090515`) and `art`: 0 world, 1 bush,
/// 2 zombies, 3 upright bushes, 4 first version. A malformed block is skipped,
/// never fatal.
fn load_local_news() -> Vec<(String, String, String, String, String, String, usize)> {
    let mut paths = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            paths.push(dir.join("news.txt"));
        }
    }
    if let Ok(appdata) = std::env::var("APPDATA") {
        paths.push(Path::new(&appdata).join("Rustium").join("news.txt"));
    }
    let Some(txt) = paths.iter().find_map(|p| std::fs::read_to_string(p).ok()) else {
        return Vec::new();
    };
    let txt = txt.trim_start_matches('\u{feff}');
    let mut blocks: Vec<Vec<&str>> = Vec::new();
    let mut cur = Vec::new();
    for line in txt.lines() {
        if line.trim() == "---" {
            blocks.push(std::mem::take(&mut cur));
        } else {
            cur.push(line);
        }
    }
    blocks.push(cur);

    let mut out = Vec::new();
    for block in blocks {
        let (mut lang, mut title, mut date, mut summary, mut body) =
            ("", String::new(), String::new(), String::new(), String::new());
        let mut version = String::new();
        let mut art = 0usize;
        for line in block {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else { continue };
            match k.trim() {
                "lang" => lang = v.trim(),
                "title" => title = v.trim().to_string(),
                "date" => date = v.trim().to_string(),
                "summary" => summary = v.trim().to_string(),
                "body" => body = v.trim().to_string(),
                "version" => version = v.trim().to_string(),
                "art" => art = v.trim().parse().unwrap_or(0).min(4),
                _ => {}
            }
        }
        if !lang.is_empty() && !title.is_empty() {
            if body.is_empty() {
                body = summary.clone();
            }
            out.push((lang.to_string(), title, date, summary, body, version, art));
            if out.len() >= 12 {
                break;
            }
        }
    }
    out
}

fn ui_texts(l: Lang) -> UiTexts {
    match l {
        Lang::En => UiTexts {
            sidebar_instances: "VERSIONS",
            instance_ready: "Ready",
            instance_replacement: "Replacement texture",
            version_label: "Launcher version",
            language_label: "LANGUAGE",
            news_shortcut: "What's new",
            folder: "Open folder",
            tab_home: "Home",
            tab_news: "News",
            news_title: "WHAT'S NEW",
            n1_title: "Rustium is now available",
            n1_short: "The launcher and the first Minecraft version are ready, with no account and nothing downloaded from Mojang.",
            n1_date: "16 Nov 2026",
            n1_body: "Rustium is here. Open the launcher, pick a language and play the first Minecraft version without creating an account. It uses the graphics from your own copy of the game and, if it finds none, draws its own.",
            n2_title: "The first Minecraft version is now in Rustium!",
            n2_short: "The first Minecraft version is now playable in Rustium, with the same feel as the original and a much higher frame rate.",
            n2_date: "16 Nov 2026",
            n2_body: "The first Minecraft version is now in Rustium. It plays like the original: press R to return to the start, build walls and corridors, fall into the void and press F3 to see your frame rate.",
            update_available: "Version {version} is available",
            update_now: "Update to {version}",
            update_downloading: "Downloading... {pct}%",
            update_ready: "Ready. Opening the installer...",
            update_page: "Release page",
            upd_no_installer: "The release has no installer attached.",
            upd_no_folder: "Cannot find a folder to store the update.",
            upd_cannot_create: "Cannot create {path}: {err}",
            upd_https_only: "The installer is not served over https.",
            upd_no_digest: "The release does not publish the installer's SHA-256.",
            upd_too_big: "The installer declares {size}, over the {limit} limit.",
            upd_download_failed: "Download failed: {err}",
            upd_cannot_write: "Cannot write {path}: {err}",
            upd_interrupted: "Download interrupted: {err}",
            upd_over_cap: "The installer is bigger than the {size} it declares.",
            upd_sha_mismatch: "The installer does not match the published SHA-256 (expected {expected}, got {got}).",
            see_more: "See more",
            see_less: "See less",
            measure_fps: "Measure FPS",
            bench_running: "Measuring... {step}/{total}",
            bench_java_wait: "Measuring Java...",
        },
        Lang::Es => UiTexts {
            sidebar_instances: "VERSIONES",
            instance_ready: "Listo",
            instance_replacement: "Textura de reemplazo",
            version_label: "Versión de Launcher",
            language_label: "IDIOMA",
            news_shortcut: "Novedades",
            folder: "Abrir carpeta",
            tab_home: "Inicio",
            tab_news: "Novedades",
            news_title: "NOVEDADES",
            n1_title: "¡Rustium ya está disponible!",
            n1_short: "El launcher y la primera versión de Minecraft ya funcionan, sin cuenta y sin descargar nada de Mojang.",
            n1_date: "16 nov 2026",
            n1_body: "Rustium ya está aquí. Abre el launcher, elige idioma y juega a la primera versión de Minecraft sin crear ninguna cuenta. Usa los gráficos de tu propia copia del juego y, si no encuentra ninguna, dibuja los suyos.",
            n2_title: "¡La primera versión de Minecraft ya está en Rustium!",
            n2_short: "La primera versión de Minecraft ya se puede jugar en Rustium, con la misma sensación que el original y muchos más fotogramas por segundo.",
            n2_date: "16 nov 2026",
            n2_body: "La primera versión de Minecraft ya está en Rustium. Se juega igual que el original: puedes volver al inicio con R, construir muros y pasillos, caer al vacío y ver los fotogramas por segundo con F3.",
            update_available: "Hay una versión nueva: {version}",
            update_now: "Actualizar a {version}",
            update_downloading: "Descargando... {pct}%",
            update_ready: "Listo. Abriendo el instalador...",
            update_page: "Página del release",
            upd_no_installer: "El release no trae instalador adjunto.",
            upd_no_folder: "No encuentro una carpeta para guardar la actualización.",
            upd_cannot_create: "No se pudo crear {path}: {err}",
            upd_https_only: "El instalador no se sirve por https.",
            upd_no_digest: "El release no publica el SHA-256 del instalador.",
            upd_too_big: "El instalador declara {size}, por encima del límite de {limit}.",
            upd_download_failed: "La descarga falló: {err}",
            upd_cannot_write: "No se pudo escribir {path}: {err}",
            upd_interrupted: "La descarga se interrumpió: {err}",
            upd_over_cap: "El instalador ocupa más que los {size} que declara.",
            upd_sha_mismatch: "El instalador no coincide con el SHA-256 publicado (esperado {expected}, obtenido {got}).",
            see_more: "Ver más",
            see_less: "Ver menos",
            measure_fps: "Medir FPS",
            bench_running: "Midiendo... {step}/{total}",
            bench_java_wait: "Midiendo Java...",
        },
        Lang::Pt => UiTexts {
            sidebar_instances: "VERSÕES",
            instance_ready: "Pronto",
            instance_replacement: "Textura substituta",
            version_label: "Versão do Launcher",
            language_label: "IDIOMA",
            news_shortcut: "Novidades",
            folder: "Abrir pasta",
            tab_home: "Início",
            tab_news: "Novidades",
            news_title: "NOVIDADES",
            n1_title: "O Rustium já está disponível!",
            n1_short: "O launcher e a primeira versão de Minecraft já funcionam, sem conta e sem descarregar nada da Mojang.",
            n1_date: "16 nov 2026",
            n1_body: "O Rustium já chegou. Abre o launcher, escolhe o idioma e joga a primeira versão de Minecraft sem criar conta. Usa os gráficos da tua própria cópia do jogo e, se não encontrar nenhum, desenha os seus.",
            n2_title: "A primeira versão do Minecraft já está no Rustium!",
            n2_short: "A primeira versão de Minecraft já se pode jogar no Rustium, com a mesma sensação do original e muitos mais fotogramas por segundo.",
            n2_date: "16 nov 2026",
            n2_body: "A primeira versão de Minecraft já está no Rustium. Joga-se como o original: podes voltar ao início com R, construir muros e corredores, cair no vazio e ver os fotogramas por segundo com F3.",
            update_available: "Há uma versão nova: {version}",
            update_now: "Atualizar para {version}",
            update_downloading: "A transferir... {pct}%",
            update_ready: "Pronto. A abrir o instalador...",
            update_page: "Página do release",
            upd_no_installer: "O lançamento não traz instalador.",
            upd_no_folder: "Não encontro uma pasta para guardar a atualização.",
            upd_cannot_create: "Não foi possível criar {path}: {err}",
            upd_https_only: "O instalador não é servido por https.",
            upd_no_digest: "O lançamento não publica o SHA-256 do instalador.",
            upd_too_big: "O instalador declara {size}, acima do limite de {limit}.",
            upd_download_failed: "A transferência falhou: {err}",
            upd_cannot_write: "Não foi possível escrever {path}: {err}",
            upd_interrupted: "A transferência foi interrompida: {err}",
            upd_over_cap: "O instalador é maior do que os {size} que declara.",
            upd_sha_mismatch: "O instalador não corresponde ao SHA-256 publicado (esperado {expected}, obtido {got}).",
            see_more: "Ver mais",
            see_less: "Ver menos",
            measure_fps: "Medir FPS",
            bench_running: "A medir... {step}/{total}",
            bench_java_wait: "A medir Java...",
        },
        Lang::Fr => UiTexts {
            sidebar_instances: "VERSIONS",
            instance_ready: "Prêt",
            instance_replacement: "Texture de remplacement",
            version_label: "Version du launcher",
            language_label: "LANGUE",
            news_shortcut: "Nouveautés",
            folder: "Ouvrir le dossier",
            tab_home: "Accueil",
            tab_news: "Nouveautés",
            news_title: "NOUVEAUTÉS",
            n1_title: "Rustium est maintenant disponible !",
            n1_short: "Le launcher et la première version de Minecraft fonctionnent, sans compte et sans rien télécharger de Mojang.",
            n1_date: "16 nov. 2026",
            n1_body: "Rustium est là. Ouvre le launcher, choisis une langue et joue à la première version de Minecraft sans créer de compte. Il utilise les graphismes de ta propre copie du jeu et, s'il n'en trouve aucun, dessine les siens.",
            n2_title: "La première version de Minecraft est dans Rustium !",
            n2_short: "La première version de Minecraft est maintenant jouable dans Rustium, avec les mêmes sensations que l'original et beaucoup plus d'images par seconde.",
            n2_date: "16 nov. 2026",
            n2_body: "La première version de Minecraft est maintenant dans Rustium. Elle se joue comme l'original : appuie sur R pour revenir au départ, construis des murs et des couloirs, tombe dans le vide et appuie sur F3 pour voir les images par seconde.",
            update_available: "Une nouvelle version est disponible : {version}",
            update_now: "Mettre à jour vers {version}",
            update_downloading: "Téléchargement... {pct}%",
            update_ready: "Prêt. Ouverture de l'installateur...",
            update_page: "Page de la version",
            upd_no_installer: "La version ne fournit pas d'installateur.",
            upd_no_folder: "Impossible de trouver un dossier pour l'actualisation.",
            upd_cannot_create: "Impossible de créer {path} : {err}",
            upd_https_only: "L'installateur n'est pas servi en https.",
            upd_no_digest: "La version ne publie pas le SHA-256 de l'installateur.",
            upd_too_big: "L'installateur annonce {size}, au-delà de la limite de {limit}.",
            upd_download_failed: "Le téléchargement a échoué : {err}",
            upd_cannot_write: "Impossible d'écrire {path} : {err}",
            upd_interrupted: "Téléchargement interrompu : {err}",
            upd_over_cap: "L'installateur est plus gros que les {size} annoncés.",
            upd_sha_mismatch: "L'installateur ne correspond pas au SHA-256 publié (attendu {expected}, obtenu {got}).",
            see_more: "Voir plus",
            see_less: "Voir moins",
            measure_fps: "Mesurer les FPS",
            bench_running: "Mesure... {step}/{total}",
            bench_java_wait: "Mesure de Java...",
        },
        Lang::De => UiTexts {
            sidebar_instances: "VERSIONEN",
            instance_ready: "Bereit",
            instance_replacement: "Ersatztextur",
            version_label: "Launcher-Version",
            language_label: "SPRACHE",
            news_shortcut: "Neuigkeiten",
            folder: "Ordner öffnen",
            tab_home: "Start",
            tab_news: "Neuigkeiten",
            news_title: "NEUIGKEITEN",
            n1_title: "Rustium ist jetzt verfügbar!",
            n1_short: "Der Launcher und die erste Minecraft-Version funktionieren, ohne Konto und ohne Download von Mojang.",
            n1_date: "16. Nov. 2026",
            n1_body: "Rustium ist da. Öffne den Launcher, wähle eine Sprache und spiele die erste Minecraft-Version, ohne ein Konto zu erstellen. Er nutzt die Grafik deiner eigenen Spielkopie und zeichnet eigene, wenn er keine findet.",
            n2_title: "Die erste Minecraft-Version ist jetzt in Rustium!",
            n2_short: "Die erste Minecraft-Version ist jetzt in Rustium spielbar, mit demselben Gefühl wie das Original und viel mehr Bildern pro Sekunde.",
            n2_date: "16. Nov. 2026",
            n2_body: "Die erste Minecraft-Version ist jetzt in Rustium. Sie spielt sich wie das Original: Mit R kehrst du zum Start zurück, du baust Wände und Gänge, fällst ins Leere und siehst mit F3 die Bilder pro Sekunde.",
            update_available: "Version {version} ist verfügbar",
            update_now: "Auf {version} aktualisieren",
            update_downloading: "Wird geladen... {pct}%",
            update_ready: "Bereit. Installer wird geöffnet...",
            update_page: "Release-Seite",
            upd_no_installer: "Die Version enthält keinen Installer.",
            upd_no_folder: "Kein Ordner für die Aktualisierung gefunden.",
            upd_cannot_create: "Konnte {path} nicht anlegen: {err}",
            upd_https_only: "Der Installer wird nicht über https ausgeliefert.",
            upd_no_digest: "Die Version veröffentlicht keinen SHA-256 für den Installer.",
            upd_too_big: "Der Installer nennt {size}, über der Grenze von {limit}.",
            upd_download_failed: "Der Download ist fehlgeschlagen: {err}",
            upd_cannot_write: "Konnte {path} nicht schreiben: {err}",
            upd_interrupted: "Der Download wurde unterbrochen: {err}",
            upd_over_cap: "Der Installer ist größer als die {size}, die er nennt.",
            upd_sha_mismatch: "Der Installer passt nicht zum veröffentlichten SHA-256 (erwartet {expected}, erhalten {got}).",
            see_more: "Mehr anzeigen",
            see_less: "Weniger anzeigen",
            measure_fps: "FPS messen",
            bench_running: "Messung... {step}/{total}",
            bench_java_wait: "Java wird gemessen...",
        },
    }
}

// ------------------------------------------------------------------- settings

fn config_path() -> Option<PathBuf> {
    let base = std::env::var("APPDATA")
        .ok()
        .map(PathBuf::from)
        .or_else(|| std::env::var("HOME").ok().map(|h| Path::new(&h).join(".config")))?;
    Some(base.join("Rustium").join("config.txt"))
}

fn saved_lang() -> Option<Lang> {
    let txt = std::fs::read_to_string(config_path()?).ok()?;
    for line in txt.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("lang=") {
            return Lang::from_code(v);
        }
    }
    None
}

/// The build chosen last time, so the picker survives a restart.
fn saved_game() -> Option<GameVersion> {
    let txt = std::fs::read_to_string(config_path()?).ok()?;
    txt.lines()
        .map(str::trim)
        .find_map(|l| l.strip_prefix("game="))
        .and_then(GameVersion::from_id)
}

/// Language and the chosen build share one tiny file, so writing either keeps
/// both. Older versions also wrote `inst=` lines for texture instances; they are
/// simply ignored now that the rail lists the five versions themselves.
fn save_config(l: Lang, game: GameVersion) {
    let Some(p) = config_path() else { return };
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let out = format!("lang={}\ngame={}\n", l.code(), game.id());
    let _ = std::fs::write(&p, out);
}

#[cfg(windows)]
fn system_language() -> Option<Lang> {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetUserDefaultUILanguage() -> u16;
    }
    let id = unsafe { GetUserDefaultUILanguage() } & 0x3ff;
    match id {
        0x09 => Some(Lang::En),
        0x0a => Some(Lang::Es),
        0x16 => Some(Lang::Pt),
        0x0c => Some(Lang::Fr),
        0x07 => Some(Lang::De),
        _ => None,
    }
}

#[cfg(not(windows))]
fn system_language() -> Option<Lang> {
    None
}

/// Saved choice, then RUSTIUM_LANG / LANG / LC_ALL, then the operating system, then
/// English.
fn detect_lang() -> Lang {
    if let Ok(v) = std::env::var("RUSTIUM_LANG") {
        if let Some(l) = Lang::from_code(&v) {
            return l;
        }
    }
    if let Some(l) = saved_lang() {
        return l;
    }
    for var in ["LANG", "LC_ALL"] {
        if let Ok(v) = std::env::var(var) {
            if let Some(l) = Lang::from_code(&v) {
                return l;
            }
        }
    }
    system_language().unwrap_or(Lang::En)
}

// ------------------------------------------------------------------- updates

/// Biggest installer the launcher will download: a release asset is about 11 MB,
/// so anything past this is not the file we published.
const MAX_SETUP_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Clone)]
struct Update {
    version: String,
    url: String,
    setup_url: Option<String>,
    setup_size: u64,
    /// SHA-256 the release publishes for the installer ("digest": "sha256:…").
    setup_sha256: Option<String>,
}

fn version_parts(v: &str) -> Vec<u32> {
    v.trim()
        .trim_start_matches(&['v', 'V'][..])
        .split(&['.', '-', '+'][..])
        .map(|p| p.parse::<u32>().unwrap_or(0))
        .collect()
}

fn is_newer(remote: &str, local: &str) -> bool {
    let (a, b) = (version_parts(remote), version_parts(local));
    for i in 0..a.len().max(b.len()) {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        if x != y {
            return x > y;
        }
    }
    false
}

/// Pulls a `"key": "value"` string out of a tiny JSON document without pulling in
/// a JSON library.
fn json_field(body: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let start = body.find(&needle)? + needle.len();
    let rest = &body[start..];
    let colon = rest.find(':')?;
    let rest = rest[colon + 1..].trim_start();
    let rest = rest.strip_prefix('"')?;
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(out),
            '\\' => {
                if let Some(n) = chars.next() {
                    out.push(n);
                }
            }
            _ => out.push(c),
        }
    }
    None
}

/// Only https, plus http when the host is this machine, so the update path can be
/// exercised against a local server without weakening what a real release may use.
fn https_ok(url: &str) -> bool {
    let u = url.trim();
    if u.is_empty() || u.len() > 2048 || u.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return false;
    }
    if let Some(rest) = u.strip_prefix("https://") {
        return !rest.is_empty();
    }
    match u.strip_prefix("http://") {
        Some(rest) => is_loopback(rest),
        None => false,
    }
}

/// True when the authority after the scheme points at this machine. The host has
/// to be exactly a loopback name: `localhost.example.com` is somebody else's
/// server, and `127.0.0.1.example` is not an address at all.
fn is_loopback(rest: &str) -> bool {
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    // Drop any `user:pass@` in front of the host.
    let authority = match authority.rfind('@') {
        Some(i) => &authority[i + 1..],
        None => authority,
    };
    let (host, port) = match authority.strip_prefix('[') {
        Some(v6) => match v6.split_once(']') {
            Some((h, tail)) if tail.is_empty() => (h, ""),
            Some((h, tail)) => match tail.strip_prefix(':') {
                Some(p) => (h, p),
                None => return false,
            },
            None => return false,
        },
        None => match authority.rsplit_once(':') {
            Some((h, p)) => (h, p),
            None => (authority, ""),
        },
    };
    if !port.is_empty() && port.parse::<u16>().is_err() {
        return false;
    }
    host.eq_ignore_ascii_case("localhost") || host == "127.0.0.1" || host == "::1"
}

/// Reads a bare number (`"size": 1037`) from the same tiny JSON: `json_field` only
/// understands quoted strings.
fn json_number(body: &str, key: &str) -> Option<u64> {
    let needle = format!("\"{key}\"");
    let start = body.find(&needle)? + needle.len();
    let rest = &body[start..];
    let colon = rest.find(':')?;
    let rest = rest[colon + 1..].trim_start();
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

/// The SHA-256 GitHub attaches to a release asset, as 64 hex digits.
fn digest_of(body: &str) -> Option<String> {
    let mut from = 0;
    while let Some(at) = body[from..].find("\"digest\"") {
        let mut rest = &body[from + at + 8..];
        from += at + 8;
        let Some(colon) = rest.find(':') else { continue };
        rest = rest[colon + 1..].trim_start();
        let Some(rest) = rest.strip_prefix('"') else { continue };
        let value: String = rest.chars().take_while(|c| *c != '"').collect();
        let hex = value.trim().trim_start_matches("sha256:").to_ascii_lowercase();
        if hex.len() == 64 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Some(hex);
        }
    }
    None
}

/// Every object inside the release's `"assets": [ … ]` array, cut out by counting
/// braces and skipping what is inside strings. Each asset is handed over whole, so
/// its `size` and its `digest` are read from the same object no matter where GitHub
/// prints them: `digest` comes before `browser_download_url`, and looking forward
/// from the url alone never saw it.
fn asset_objects(body: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let Some(at) = body.find("\"assets\"") else {
        return out;
    };
    let Some(open) = body[at..].find('[') else {
        return out;
    };
    let base = at + open;
    let mut depth = 0usize;
    let mut start = 0usize;
    let mut in_str = false;
    let mut escape = false;
    for (i, &c) in body[base..].as_bytes().iter().enumerate() {
        if in_str {
            if escape {
                escape = false;
            } else if c == b'\\' {
                escape = true;
            } else if c == b'"' {
                in_str = false;
            }
            continue;
        }
        match c {
            b'"' => in_str = true,
            b'{' => {
                if depth == 0 {
                    start = i;
                }
                depth += 1;
            }
            b'}' => {
                if depth > 0 {
                    depth -= 1;
                    if depth == 0 {
                        out.push(&body[base + start..base + i + 1]);
                    }
                }
            }
            b']' if depth == 0 => break,
            _ => {}
        }
    }
    out
}

/// The asset that describes the installer. The `.exe` wins, so a notes file or a
/// zip published next to it never gets taken for the installer, and only https is
/// accepted.
fn installer_asset(body: &str) -> Option<(&str, String)> {
    let mut fallback: Option<(&str, String)> = None;
    for asset in asset_objects(body) {
        let Some(url) = json_field(asset, "browser_download_url").filter(|u| https_ok(u)) else {
            continue;
        };
        if url.to_ascii_lowercase().ends_with(".exe") {
            return Some((asset, url));
        }
        if fallback.is_none() {
            fallback = Some((asset, url));
        }
    }
    fallback
}

/// Reads the release JSON: version, installer (only over https), its size and the
/// digest it publishes. Everything the updater needs to decide, without running
/// anything.
fn parse_release(body: &str) -> Option<Update> {
    let version = json_field(body, "tag_name")?;
    let url = json_field(body, "html_url")
        .filter(|u| https_ok(u))
        .unwrap_or_else(|| UPDATE_URL.to_string());
    let (asset, setup_url) = match installer_asset(body) {
        Some((a, u)) => (a, Some(u)),
        None => ("", None),
    };
    Some(Update {
        version,
        url,
        setup_url,
        setup_size: json_number(asset, "size").unwrap_or(0),
        setup_sha256: digest_of(asset),
    })
}

/// Asks the project's release list whether there is something newer. Returns None
/// on any failure: no network, no releases yet, or a malformed answer. The UI never
/// shows an error for this.
fn check_update() -> Option<Update> {
    let url = std::env::var("RUSTIUM_UPDATE_URL").unwrap_or_else(|_| UPDATE_URL.to_string());
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(4))
        .build();
    let body = agent
        .get(&url)
        .set("User-Agent", &format!("{NAME}/{VERSION}"))
        .call()
        .ok()?
        .into_string()
        .ok()?;
    let found = parse_release(&body)?;
    if !is_newer(&found.version, VERSION) {
        return None;
    }
    Some(found)
}

/// Opens a link with the system handler. On Windows this goes through
/// ShellExecuteW instead of cmd.exe: a URL is data, and cmd would read a `&` or a
/// `|` inside it as just another command.
fn open_in_browser(url: &str) {
    if !https_ok(url) {
        return;
    }
    #[cfg(windows)]
    {
        fn wide(s: &str) -> Vec<u16> {
            s.encode_utf16().chain(std::iter::once(0)).collect()
        }
        #[link(name = "shell32")]
        extern "system" {
            fn ShellExecuteW(
                hwnd: *mut std::ffi::c_void,
                op: *const u16,
                file: *const u16,
                params: *const u16,
                dir: *const u16,
                show: i32,
            ) -> isize;
        }
        let op = wide("open");
        let link = wide(url.trim());
        // 1 = SW_SHOWNORMAL
        unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                op.as_ptr(),
                link.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                1,
            );
        }
    }
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
}

// --------------------------------------------------------------------- sha256
/// SHA-256 of the downloaded installer, so nothing is executed before the bytes
/// match the digest the release publishes. Written out here instead of pulling in
/// a crypto crate: the launcher only needs this one hash.
mod sha256 {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    const H0: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    pub struct Sha256 {
        h: [u32; 8],
        buf: [u8; 64],
        used: usize,
        len: u64,
    }

    impl Sha256 {
        pub fn new() -> Self {
            Sha256 { h: H0, buf: [0; 64], used: 0, len: 0 }
        }

        pub fn update(&mut self, data: &[u8]) {
            self.len = self.len.wrapping_add(data.len() as u64);
            self.feed(data);
        }

        fn feed(&mut self, mut data: &[u8]) {
            if self.used > 0 {
                let take = (64 - self.used).min(data.len());
                self.buf[self.used..self.used + take].copy_from_slice(&data[..take]);
                self.used += take;
                data = &data[take..];
                if self.used == 64 {
                    let block = self.buf;
                    self.compress(&block);
                    self.used = 0;
                }
            }
            while data.len() >= 64 {
                let mut block = [0u8; 64];
                block.copy_from_slice(&data[..64]);
                self.compress(&block);
                data = &data[64..];
            }
            if !data.is_empty() {
                self.buf[..data.len()].copy_from_slice(data);
                self.used = data.len();
            }
        }

        /// Padding, then the digest as 64 lowercase hex digits.
        pub fn hex(mut self) -> String {
            let bits = self.len.wrapping_mul(8).to_be_bytes();
            let zeros = if self.used < 56 { 56 - self.used } else { 120 - self.used };
            let mut tail = vec![0u8; zeros + 8];
            tail[0] = 0x80;
            tail[zeros..].copy_from_slice(&bits);
            self.feed(&tail);
            let mut out = String::with_capacity(64);
            for word in self.h {
                out.push_str(&format!("{word:08x}"));
            }
            out
        }

        fn compress(&mut self, block: &[u8; 64]) {
            let mut w = [0u32; 64];
            for i in 0..16 {
                w[i] = u32::from_be_bytes([
                    block[i * 4],
                    block[i * 4 + 1],
                    block[i * 4 + 2],
                    block[i * 4 + 3],
                ]);
            }
            for i in 16..64 {
                let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
                let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
                w[i] = w[i - 16]
                    .wrapping_add(s0)
                    .wrapping_add(w[i - 7])
                    .wrapping_add(s1);
            }
            let mut a = self.h[0];
            let mut b = self.h[1];
            let mut c = self.h[2];
            let mut d = self.h[3];
            let mut e = self.h[4];
            let mut f = self.h[5];
            let mut g = self.h[6];
            let mut h = self.h[7];
            for i in 0..64 {
                let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
                let ch = (e & f) ^ ((!e) & g);
                let t1 = h
                    .wrapping_add(s1)
                    .wrapping_add(ch)
                    .wrapping_add(K[i])
                    .wrapping_add(w[i]);
                let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
                let maj = (a & b) ^ (a & c) ^ (b & c);
                let t2 = s0.wrapping_add(maj);
                h = g;
                g = f;
                f = e;
                e = d.wrapping_add(t1);
                d = c;
                c = b;
                b = a;
                a = t1.wrapping_add(t2);
            }
            for (slot, value) in self.h.iter_mut().zip([a, b, c, d, e, f, g, h]) {
                *slot = slot.wrapping_add(value);
            }
        }
    }
}

// ----------------------------------------------------------------------- main

fn main() {
    let opts = match options() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("{NAME}: {e}");
            std::process::exit(2);
        }
    };
    let t = texts(opts.lang);
    if opts.help {
        help(&t);
        return;
    }
    if !opts.console {
        match ui::gui(opts.clone()) {
            Ok(()) => return,
            Err(e) => {
                eprintln!("{NAME}: {e}");
                eprintln!("  falling back to console mode");
            }
        }
    }
    banner(&t);
    if let Err(e) = run(&opts) {
        eprintln!();
        eprintln!("  ERROR: {e}");
        std::process::exit(1);
    }
}

fn run(opts: &Options) -> R<()> {
    let t = texts(opts.lang);
    let det = detect_atlas(opts);
    match &det.path {
        Some(p) => say!(
            "{}",
            fill(
                &fill(&t.tex_found, "{path}", &p.display().to_string()),
                "{source}",
                det.source(&t)
            )
        ),
        None => {
            say!(
                "{}",
                fill(&t.tex_none, "{version}", GAME_VERSION)
            );
            say!("{}", t.tex_replacement);
            if !det.origin.is_empty() {
                say!("  ({})", det.origin);
            }
        }
    }
    if opts.status {
        return Ok(());
    }
    launch_game(det.path.as_deref(), &t, opts.game)
}

// ---------------------------------------------------------------- arguments

fn options() -> R<Options> {
    let mut opts = Options {
        dir_minecraft: minecraft_dir()?,
        atlas: None,
        lang: Lang::En,
        game: GameVersion::Rd132211,
        status: false,
        help: false,
        console: false,
    };
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--status" | "--estado" => opts.status = true,
            "--game" | "--juego" => {
                i += 1;
                let id = args.get(i).ok_or("--game needs a build (rd-132211, rd-132328, rd-20090515, rd-160052, rd-161348)")?;
                opts.game = GameVersion::from_id(id)
                    .ok_or_else(|| format!("unknown build: {id}"))?;
            }
            "--console" => opts.console = true,
            "--atlas" => {
                i += 1;
                let r = args.get(i).ok_or("--atlas needs a path (.jar or .png)")?;
                opts.atlas = Some(PathBuf::from(r));
            }
            "--dir" => {
                i += 1;
                let d = args.get(i).ok_or("--dir needs a folder")?;
                opts.dir_minecraft = PathBuf::from(d);
            }
            "--lang" => {
                i += 1;
                let code = args.get(i).ok_or("--lang needs a code (en, es, pt, fr, de)")?;
                opts.lang = Lang::from_code(code)
                    .ok_or_else(|| format!("unknown language: {code}"))?;
            }
            "--help" | "-h" | "--ayuda" => opts.help = true,
            other => return Err(format!("unknown option: {other} (try --help)")),
        }
        i += 1;
    }
    opts.lang = if args.iter().any(|a| a == "--lang") {
        opts.lang
    } else {
        detect_lang()
    };
    if !args.iter().any(|a| a == "--game" || a == "--juego") {
        if let Some(g) = saved_game() {
            opts.game = g;
        }
    }
    Ok(opts)
}

fn help(t: &Texts) {
    say!(
        "{}",
        fill(&fill(t.help, "{name}", NAME), "{version}", VERSION)
    );
}

fn banner(t: &Texts) {
    say!("==============================================");
    say!("  {NAME} {VERSION} - RubyDung RS");
    say!("  {}", t.tagline);
    say!("==============================================");
    say!();
}

// -------------------------------------------------------------------- paths

fn minecraft_dir() -> R<PathBuf> {
    if let Ok(a) = std::env::var("RUSTIUM_MC") {
        return Ok(PathBuf::from(a));
    }
    if let Ok(a) = std::env::var("APPDATA") {
        return Ok(Path::new(&a).join(".minecraft"));
    }
    if let Ok(h) = std::env::var("HOME") {
        return Ok(Path::new(&h).join(".minecraft"));
    }
    Err("cannot find the .minecraft folder (set RUSTIUM_MC)".into())
}

// ------------------------------------------------------------------ textures
// Everything about textures is local: the only network call lives in check_update.

#[derive(Clone)]
struct Detection {
    path: Option<PathBuf>,
    /// `detected`, `manual`, or a description of what is wrong with a given path.
    origin: String,
}

impl Detection {
    fn source(&self, t: &Texts) -> &'static str {
        if self.origin == "manual" {
            t.manual
        } else {
            t.detected
        }
    }

    fn summary(&self, t: &Texts) -> String {
        match &self.path {
            Some(p) => format!("{} ({})", p.display(), self.source(t)),
            None => fill(&t.tex_none, "{version}", GAME_VERSION),
        }
    }
}

/// True for a .jar that carries `terrain.png`, or for a .png (the game checks the
/// size when it loads it).
fn atlas_ok(path: &Path) -> bool {
    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(_) => return false,
    };
    if data.starts_with(b"PK") {
        match zip::ZipArchive::new(Cursor::new(data)) {
            Ok(mut a) => a.by_name("terrain.png").map(|e| e.size() > 0).unwrap_or(false),
            Err(_) => false,
        }
    } else {
        data.starts_with(b"\x89PNG")
    }
}

fn push_copy(out: &mut Vec<PathBuf>, p: PathBuf) {
    if atlas_ok(&p) && !out.contains(&p) {
        out.push(p);
    }
}

/// Valid copies of one build in the normal installation and in the instances of
/// the most used launchers (each instance has its own .minecraft).
fn installed_copies(base: &Path, id: &str) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let jar = format!("{id}.jar");
    push_copy(&mut out, base.join("versions").join(id).join(&jar));
    let mut roots: Vec<PathBuf> = vec![base.join("instances")];
    if let Some(roaming) = base.parent() {
        for name in ["PrismLauncher", "MultiMC", "ATLauncher", "PolyMC"] {
            roots.push(roaming.join(name).join("instances"));
        }
    }
    for root in roots {
        let Ok(entries) = std::fs::read_dir(&root) else { continue };
        for e in entries.flatten() {
            let dir = e.path();
            push_copy(&mut out, dir.join("minecraft").join("versions").join(id).join(&jar));
            push_copy(&mut out, dir.join(".minecraft").join("versions").join(id).join(&jar));
        }
    }
    out
}

fn detect_atlas(opts: &Options) -> Detection {
    if let Some(r) = &opts.atlas {
        return if atlas_ok(r) {
            Detection { path: Some(r.clone()), origin: "manual".to_string() }
        } else {
            Detection {
                path: None,
                origin: format!("the given path has no terrain.png: {}", r.display()),
            }
        };
    }
    let mut copies = installed_copies(&opts.dir_minecraft, GAME_VERSION);
    if !copies.is_empty() {
        return Detection { path: Some(copies.remove(0)), origin: "detected".to_string() };
    }
    let loose = opts.dir_minecraft.join("versions").join(GAME_VERSION).join("terrain.png");
    if atlas_ok(&loose) {
        return Detection { path: Some(loose), origin: "detected".to_string() };
    }
    Detection { path: None, origin: String::new() }
}

/// True when this computer has the textures of that build (its own jar, or a
/// loose terrain.png). The rail uses it for the ready/replacement dot without
/// decoding the jar every frame; `Window::new` fills the cache once.
fn version_textures(base: &Path, game: GameVersion) -> bool {
    if !installed_copies(base, game.id()).is_empty() {
        return true;
    }
    atlas_ok(&base.join("versions").join(game.id()).join("terrain.png"))
}

// ---------------------------------------------------------------------- game

fn launch_game(atlas: Option<&Path>, t: &Texts, game: GameVersion) -> R<()> {
    let exe = find_game(t, game)?;
    say!();
    say!("{}", fill(&t.starting, "{path}", &exe.display().to_string()));
    let mut cmd = std::process::Command::new(&exe);
    // level.dat is written next to the executable, so "Open folder" always
    // shows the world that was just saved, whatever directory the launcher was
    // started from (a shortcut, a terminal, another drive).
    let game_dir = exe
        .parent()
        .map(Path::to_path_buf)
        .or_else(|| std::env::current_dir().ok())
        .ok_or_else(|| "cannot find the game folder".to_string())?;
    cmd.current_dir(game_dir);
    // Each build reads its own terrain.png from its own jar; only the original
    // rd-132211 port takes the atlas the picker detected by hand.
    if let (Some(r), GameVersion::Rd132211) = (atlas, game) {
        cmd.env("RD_ATLAS", r);
    }
    let status = cmd
        .status()
        .map_err(|e| format!("could not start the game: {e}"))?;
    say!("{}", fill(&t.game_exited, "{status}", &status.to_string()));
    Ok(())
}

// ------------------------------------------------- measuring the Java original

/// The middle value: steadier than an average when one reading catches a hitch.
fn median(v: &mut [u32]) -> u32 {
    v.sort_unstable();
    v[v.len() / 2]
}

fn mc_home() -> PathBuf {
    std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_default()
        .join(".minecraft")
}

fn walk_collect(dir: &Path, keep: &dyn Fn(&str) -> bool, depth: usize, out: &mut Vec<PathBuf>) {
    if depth == 0 || !dir.is_dir() {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_file() {
            if let Some(n) = p.file_name().and_then(|f| f.to_str()) {
                if keep(n) {
                    out.push(p);
                }
            }
        } else if p.is_dir() {
            walk_collect(&p, keep, depth - 1, out);
        }
    }
}

/// Any installed JVM, preferring Java 8 because the 2010 game was built for it.
fn find_java() -> Option<PathBuf> {
    if let Ok(j) = std::env::var("RUSTIUM_JAVA") {
        let p = PathBuf::from(j);
        if p.exists() {
            return Some(p);
        }
    }
    let mut found = Vec::new();
    for var in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"] {
        let Ok(base) = std::env::var(var) else { continue };
        let base = PathBuf::from(base);
        for vendor in ["Eclipse Adoptium", "Java", "Zulu", "BellSoft", "Microsoft"] {
            let Ok(rd) = std::fs::read_dir(base.join(vendor)) else { continue };
            for e in rd.flatten() {
                let exe = e.path().join("bin").join("java.exe");
                if exe.exists() {
                    found.push(exe);
                }
            }
        }
    }
    found.sort_by_key(|p| {
        let s = p.display().to_string().to_lowercase();
        let eight = s.contains("jdk-8") || s.contains("jdk8") || s.contains("1.8") || s.contains("zulu-8");
        if eight { 0 } else { 1 }
    });
    found.into_iter().next()
}

/// The benchmark agent rides inside the launcher and is written out on demand.
fn agent_jar() -> Option<PathBuf> {
    let bytes: &[u8] = include_bytes!("../../assets/rustbench.jar");
    let base = std::env::var("LOCALAPPDATA").ok().map(PathBuf::from)?;
    let dir = base.join("Rustium").join("bench");
    std::fs::create_dir_all(&dir).ok()?;
    let dst = dir.join("rustbench.jar");
    // Always rewritten: this file is loaded as a Java agent, so a copy left on
    // disk by anybody else is never trusted, even at the same size.
    std::fs::write(&dst, bytes).ok()?;
    Some(dst)
}

/// Runs the player's own rd-132211 jar for a few seconds with the agent attached
/// and returns the settled frame rate. Nothing from Mojang is bundled: the jar is
/// read from the copy the player already has.
fn measure_java() -> Option<u32> {
    let mc = mc_home();
    let java = find_java()?;
    // Several hash folders under bin/ carry a lwjgl64.dll; only the complete set
    // links, so take the folder with the most native libraries.
    let mut dlls = Vec::new();
    walk_collect(&mc, &|n| n.eq_ignore_ascii_case("lwjgl64.dll"), 8, &mut dlls);
    let natives = dlls
        .iter()
        .filter_map(|p| p.parent().map(Path::to_path_buf))
        .map(|d| {
            let n = std::fs::read_dir(&d)
                .map(|rd| {
                    rd.flatten()
                        .filter(|e| e.path().extension().is_some_and(|x| x == "dll"))
                        .count()
                })
                .unwrap_or(0);
            (n, d)
        })
        .max_by_key(|(n, _)| *n)?
        .1;

    let game = mc
        .join("versions")
        .join(GAME_VERSION)
        .join(format!("{GAME_VERSION}.jar"));
    if !game.exists() {
        return None;
    }
    let agent = agent_jar()?;
    let mut libs = Vec::new();
    walk_collect(
        &mc.join("libraries"),
        &|n| {
            (n.starts_with("lwjgl-2") || n.starts_with("lwjgl_util-2") || n.starts_with("jinput-"))
                && n.ends_with(".jar")
        },
        9,
        &mut libs,
    );
    if libs.is_empty() {
        return None;
    }
    let mut cp = vec![agent.display().to_string(), game.display().to_string()];
    cp.extend(libs.iter().map(|p| p.display().to_string()));
    let cp_debug = cp.join(";");

    // A clean world for the benchmark: the game writes level.dat next to its
    // working folder, so a temp one keeps the player's own world untouched and
    // always starts at the spawn point looking at the horizon.
    let base = std::env::var("LOCALAPPDATA").ok().map(PathBuf::from)?;
    let world = base.join("Rustium").join("bench").join("world");
    let _ = std::fs::remove_dir_all(&world);
    std::fs::create_dir_all(&world).ok()?;
    {
        let data = std::fs::read(&game).ok()?;
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(data)).ok()?;
        let mut entry = zip.by_name("terrain.png").ok()?;
        let mut out = Vec::new();
        std::io::Read::read_to_end(&mut entry, &mut out).ok()?;
        std::fs::write(world.join("terrain.png"), out).ok()?;
    }

    let mut child = match std::process::Command::new(&java)
        .arg(format!("-javaagent:{}", agent.display()))
        .arg(format!("-Drustium.spawn={BENCH_SPAWN}"))
        .arg(format!("-Djava.library.path={}", natives.display()))
        .arg("-cp")
        .arg(&cp_debug)
        .arg("com.mojang.rubydung.RubyDung")
        .current_dir(&world)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return None,
    };

    let errs = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    if let Some(mut e) = child.stderr.take() {
        let buf = errs.clone();
        std::thread::spawn(move || {
            use std::io::Read;
            let mut s = String::new();
            let _ = e.read_to_string(&mut s);
            if let Ok(mut b) = buf.lock() {
                *b = s;
            }
        });
    }

    let pid = child.id();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(10));
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    });

    let mut vals: Vec<u32> = Vec::new();
    if let Some(out) = child.stdout.take() {
        use std::io::BufRead;
        for line in std::io::BufReader::new(out).lines().map_while(Result::ok) {
            if let Some(v) = line.strip_prefix("RUSTIUM_FPS=") {
                if let Ok(v) = v.trim().parse::<u32>() {
                    vals.push(v);
                }
            }
            if vals.len() >= 16 {
                break;
            }
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    // The throwaway world goes away with everything the game wrote in it.
    let _ = std::fs::remove_dir_all(&world);
    if vals.is_empty() {
        // Leave a short trace of what went wrong, for support and diagnosis.
        let why = errs.lock().map(|s| s.clone()).unwrap_or_default();
        let base = std::env::var("LOCALAPPDATA").map(PathBuf::from).unwrap_or_default();
        let dir = base.join("Rustium").join("bench");
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(
            dir.join("last.log"),
            format!(
                "java: {}\nnatives: {}\ngame: {}\ncompil:\n{}\nsalida:\n{}",
                java.display(),
                natives.display(),
                game.display(),
                cp_debug,
                why
            ),
        );
        return None;
    }
    // The last eight readings, with the JIT warm and the window settled.
    let start = (vals.len() - 8).max(vals.len() / 2);
    let mut tail = vals[start..].to_vec();
    Some(median(&mut tail))
}

fn find_game(t: &Texts, game: GameVersion) -> R<PathBuf> {
    let name = game.exe();
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(d) = exe.parent() {
            candidates.push(d.join(name));
            // target/x86_64-pc-windows-gnu/release/rustium.exe -> project root
            if let Some(root) = d.parent().and_then(|p| p.parent()).and_then(|p| p.parent()) {
                candidates.push(root.join("target").join("x86_64-pc-windows-gnu").join("release").join(name));
                candidates.push(root.join("target").join("release").join(name));
            }
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("target").join("x86_64-pc-windows-gnu").join("release").join(name));
        candidates.push(cwd.join("target").join("release").join(name));
        candidates.push(cwd.join(name));
    }
    candidates
        .into_iter()
        .find(|c| c.exists())
        .ok_or_else(|| fill(&fill(&t.no_game, "{name}", name), "{version}", ""))
}

// ------------------------------------------------------------------------- UI

mod ui {
    use super::*;
    use eframe::egui;
    use egui::{Color32, RichText};
    use std::io::{Read, Write};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::mpsc::{channel, Receiver};
    use std::sync::Arc;

    // Palette: deep background, a slightly lighter rail, rounded cards on top.
    const BG_TOP: Color32 = Color32::from_rgb(38, 18, 36);
    const BG_BOTTOM: Color32 = Color32::from_rgb(10, 7, 13);
    const RAIL: Color32 = Color32::from_rgb(24, 12, 24);
    // Frosted panels: a whisper of white over the gradient.
    const GLASS: Color32 = Color32::from_rgba_premultiplied(26, 26, 26, 26);
    const GLASS_HI: Color32 = Color32::from_rgba_premultiplied(40, 40, 40, 40);
    const EDGE: Color32 = Color32::from_rgba_premultiplied(50, 50, 50, 50);
    const SOFT: Color32 = Color32::from_rgb(190, 178, 192);
    const MUTED: Color32 = Color32::from_rgb(140, 126, 144);
    const MAGENTA: Color32 = Color32::from_rgb(255, 61, 146);
    const GREEN: Color32 = Color32::from_rgb(120, 220, 160);
    const AMBER: Color32 = Color32::from_rgb(240, 192, 102);
    const BLUE: Color32 = Color32::from_rgb(186, 168, 255);
    const TEXT: Color32 = Color32::from_rgb(245, 238, 244);
    const SELECTED: Color32 = Color32::from_rgb(52, 22, 43);

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Tab {
        Home,
        News,
    }

    enum Msg {
        Line(String),
        Done(Result<(), String>),
    }

    enum UpMsg {
        Ready(PathBuf),
        Failed(String),
    }

    /// A benchmark run of the port: the game walks through four camera angles
    /// and prints one `BENCH step=… fps=…` line per angle.
    struct BenchRun {
        rx: Receiver<String>,
        fps: Vec<u32>,
        rust_done: bool,
    }

    impl BenchRun {
        /// Median of the readings so far, once there is at least one.
        fn settled(&self) -> Option<u32> {
            let mut v = self.fps.clone();
            if v.is_empty() {
                None
            } else {
                Some(median(&mut v))
            }
        }
    }

    const BENCH_STEPS: usize = 5;

    /// "Size in the launcher's own words": MB from one up, kB below that, so a small
    /// test installer never reads "0 MB".
    fn size_text(bytes: u64) -> String {
        if bytes >= 1024 * 1024 {
            format!("{} MB", bytes / (1024 * 1024))
        } else {
            format!("{} kB", (bytes + 1023) / 1024)
        }
    }

    /// A message about a file, with `{path}` and `{err}` filled in.
    fn path_fail(template: &str, path: &Path, e: &std::io::Error) -> String {
        fill(
            &fill(template, "{path}", &path.display().to_string()),
            "{err}",
            &e.to_string(),
        )
    }

    /// Downloads the installer and reports progress through the atomic counter.
    /// Nothing it returns is run: the caller only spawns a file that got here with
    /// https, inside the size limit and matching the digest the release published.
    fn download_setup(u: &Update, progress: &Arc<AtomicU64>, lang: Lang) -> Result<PathBuf, String> {
        let t = ui_texts(lang);
        let url = u
            .setup_url
            .clone()
            .ok_or_else(|| t.upd_no_installer.to_string())?;
        let dir = update_dir().ok_or_else(|| t.upd_no_folder.to_string())?;
        std::fs::create_dir_all(&dir).map_err(|e| path_fail(t.upd_cannot_create, &dir, &e))?;
        // A release has to say what the file is before it is run: https, a size we
        // can cap and a SHA-256 to check the bytes against.
        if !https_ok(&url) {
            return Err(t.upd_https_only.to_string());
        }
        let expected = u
            .setup_sha256
            .clone()
            .ok_or_else(|| t.upd_no_digest.to_string())?;
        if u.setup_size > MAX_SETUP_BYTES {
            return Err(fill(
                &fill(t.upd_too_big, "{size}", &size_text(u.setup_size)),
                "{limit}",
                &size_text(MAX_SETUP_BYTES),
            ));
        }
        // Trust the declared size as the ceiling: a stream that keeps going past it
        // is not the file we published.
        let cap = if u.setup_size > 0 { u.setup_size } else { MAX_SETUP_BYTES };
        let dst = dir.join("Rustium-setup.exe");
        let resp = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .get(&url)
            .set("User-Agent", &format!("{NAME}/{VERSION}"))
            .call()
            .map_err(|e| fill(t.upd_download_failed, "{err}", &e.to_string()))?;
        let mut reader = resp.into_reader();
        let mut file = std::fs::File::create(&dst).map_err(|e| path_fail(t.upd_cannot_write, &dst, &e))?;
        let mut hasher = sha256::Sha256::new();
        let mut buf = [0u8; 32 * 1024];
        let mut total: u64 = 0;
        loop {
            let n = reader
                .read(&mut buf)
                .map_err(|e| fill(t.upd_interrupted, "{err}", &e.to_string()))?;
            if n == 0 {
                break;
            }
            total += n as u64;
            if total > cap {
                drop(file);
                let _ = std::fs::remove_file(&dst);
                return Err(fill(t.upd_over_cap, "{size}", &size_text(cap)));
            }
            hasher.update(&buf[..n]);
            file.write_all(&buf[..n])
                .map_err(|e| path_fail(t.upd_cannot_write, &dst, &e))?;
            progress.fetch_add(n as u64, Ordering::Relaxed);
        }
        drop(file);
        let got = hasher.hex();
        if got != expected {
            let _ = std::fs::remove_file(&dst);
            return Err(fill(
                &fill(t.upd_sha_mismatch, "{expected}", &expected),
                "{got}",
                &got,
            ));
        }
        Ok(dst)
    }

    fn update_dir() -> Option<PathBuf> {
        let base = std::env::var("LOCALAPPDATA")
            .ok()
            .map(PathBuf::from)
            .or_else(|| std::env::var("APPDATA").ok().map(PathBuf::from))?;
        Some(base.join("Rustium").join("update"))
    }

    /// A soft chime when a measurement finishes. The WAV goes in the binary and
    /// plays straight from memory, so there is no audio dependency and no temp
    /// file on disk.
    #[cfg(windows)]
    fn beep() {
        #[link(name = "winmm")]
        extern "system" {
            fn PlaySound(data: *const u8, module: usize, flags: u32) -> i32;
        }
        const SND_ASYNC: u32 = 0x0001;
        const SND_NODEFAULT: u32 = 0x0002;
        const SND_MEMORY: u32 = 0x0004;
        let wav: &[u8] = include_bytes!("../../assets/chime.wav");
        unsafe {
            PlaySound(wav.as_ptr(), 0, SND_MEMORY | SND_ASYNC | SND_NODEFAULT);
        }
    }

    #[cfg(not(windows))]
    fn beep() {}

    fn reveal_dir(dir: &Path) {
        #[cfg(windows)]
        let _ = std::process::Command::new("explorer").arg(dir).spawn();
        #[cfg(not(windows))]
        let _ = std::process::Command::new("xdg-open").arg(dir).spawn();
    }

    /// Pixel face for the wordmark and the big title (Press Start 2P, OFL).
    ///
    /// The glyphs are drawn on an 8 px grid, so the requested size is turned into
    /// whole multiples of 8 *device* pixels; otherwise the strokes land between
    /// pixels and the letters look chewed at any DPI.
    fn pixel(ctx: &egui::Context, device_px: f32) -> egui::FontId {
        let ppp = ctx.pixels_per_point().max(0.5);
        let quantised = (device_px / 8.0).round().max(1.0) * 8.0;
        egui::FontId::new(quantised / ppp, egui::FontFamily::Name("pixel".into()))
    }

    /// Window and taskbar icon: the pixel R, stored as raw RGBA so no image
    /// decoder is needed.
    fn app_icon() -> egui::IconData {
        let bytes: &[u8] = include_bytes!("../../assets/logo.rgba");
        let w = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let h = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        egui::IconData { rgba: bytes[8..].to_vec(), width: w, height: h }
    }

    fn load_fonts(ctx: &egui::Context) {
        let mut fonts = egui::FontDefinitions::default();
        let pixel: &[u8] = include_bytes!("../../assets/fonts/PressStart2P-Regular.ttf");
        let display: &[u8] = include_bytes!("../../assets/fonts/LilitaOne-Regular.ttf");
        let body: &[u8] = include_bytes!("../../assets/fonts/VarelaRound-Regular.ttf");
        fonts
            .font_data
            .insert("pixel".to_owned(), egui::FontData::from_static(pixel).into());
        fonts
            .font_data
            .insert("display".to_owned(), egui::FontData::from_static(display).into());
        fonts
            .font_data
            .insert("body".to_owned(), egui::FontData::from_static(body).into());
        fonts
            .families
            .entry(egui::FontFamily::Name("pixel".into()))
            .or_default()
            .insert(0, "pixel".to_owned());
        fonts
            .families
            .entry(egui::FontFamily::Name("display".into()))
            .or_default()
            .insert(0, "display".to_owned());
        // The rounded face carries the normal text; the built-ins stay as fallback.
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "body".to_owned());
        ctx.set_fonts(fonts);
    }

    /// Chunky display face for titles, tabs and buttons (Lilita One, OFL).
    fn display(size: f32) -> egui::FontId {
        egui::FontId::new(size, egui::FontFamily::Name("display".into()))
    }

    fn disp(text: impl Into<String>, size: f32) -> egui::RichText {
        egui::RichText::new(text).font(display(size))
    }

    /// Symmetric ease, used when something shrinks: it starts and ends smooth.
    fn smooth_step(t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }

    /// Slight overshoot at the end, so expansions feel springy.
    fn ease_out_back(t: f32) -> f32 {
        let c1 = 1.70158_f32;
        let c3 = c1 + 1.0;
        let u = t.clamp(0.0, 1.0) - 1.0;
        1.0 + c3 * u * u * u + c1 * u * u
    }

    /// Height of a damped bounce after the logo touches the centre. The first
    /// rebound is the visible one; the rest die away quickly.
    fn logo_bounce(t: f32) -> f32 {
        if t <= 0.0 {
            0.0
        } else {
            (t * std::f32::consts::PI / 0.27).sin().abs() * 46.0 * (-3.1 * t).exp()
        }
    }

    fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
        let t = t.clamp(0.0, 1.0);
        let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round().clamp(0.0, 255.0) as u8;
        Color32::from_rgba_unmultiplied(
            f(a.r(), b.r()),
            f(a.g(), b.g()),
            f(a.b(), b.b()),
            f(a.a(), b.a()),
        )
    }

    /// Paints pixel-face text without letting a label clip it (the glyphs are
    /// taller than a normal line box).
    fn pixel_text(ui: &mut egui::Ui, text: &str, device_px: f32, color: Color32) -> egui::Rect {
        let galley = ui
            .painter()
            .layout_no_wrap(text.to_string(), pixel(ui.ctx(), device_px), color);
        let (rect, _) = ui.allocate_exact_size(galley.size(), egui::Sense::hover());
        ui.painter().galley(rect.min, galley, color);
        rect
    }

    fn glow(ui: &egui::Ui, center: egui::Pos2, radius: f32, tint: Color32) {
        // Layered translucent circles read as a soft blur.
        for (i, alpha) in [7u8, 6, 5, 4, 3].iter().enumerate() {
            let f = 1.0 - i as f32 * 0.16;
            ui.painter().circle_filled(
                center,
                radius * f,
                Color32::from_rgba_unmultiplied(tint.r(), tint.g(), tint.b(), *alpha),
            );
        }
    }

    fn backdrop(ui: &egui::Ui) {
        let r = ui.max_rect();
        let mut m = egui::Mesh::default();
        m.colored_vertex(r.left_top(), BG_TOP);
        m.colored_vertex(r.right_top(), Color32::from_rgb(28, 14, 30));
        m.colored_vertex(r.left_bottom(), BG_BOTTOM);
        m.colored_vertex(r.right_bottom(), BG_BOTTOM);
        m.add_triangle(0, 1, 2);
        m.add_triangle(1, 3, 2);
        ui.painter().add(egui::Shape::Mesh(m.into()));
        // Two glows so the background is not a flat gradient.
        glow(
            ui,
            r.left_top() + egui::vec2(r.width() * 0.02, r.height() * 0.55),
            620.0,
            MAGENTA,
        );
        glow(
            ui,
            r.right_top() + egui::vec2(r.width() * 0.05, r.height() * 0.02),
            520.0,
            Color32::from_rgb(150, 90, 255),
        );
        // A very quiet R anchors the empty lower-right corner without adding
        // another panel to the layout.
        let mark = Color32::from_rgba_unmultiplied(255, 61, 146, 8);
        let g = ui
            .painter()
            .layout_no_wrap("R".to_owned(), pixel(ui.ctx(), 420.0), mark);
        let at = egui::pos2(
            r.right() - g.size().x - 46.0,
            r.bottom() - g.size().y - 18.0,
        );
        ui.painter().galley(at, g, mark);
    }

    fn card(ui: &mut egui::Ui, body: impl FnOnce(&mut egui::Ui)) {
        egui::Frame::NONE
            .fill(GLASS)
            .stroke(egui::Stroke::new(1.0_f32, EDGE))
            .corner_radius(16)
            .inner_margin(egui::Margin::symmetric(18, 14))
            .show(ui, body);
    }

    fn plain_button(ui: &mut egui::Ui, text: &str, width: f32, enabled: bool) -> egui::Response {
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 44.0), egui::Sense::click());
        let hovered = enabled && resp.hovered();
        let ctx = ui.ctx().clone();
        let t = ctx.animate_bool_with_time(resp.id.with("hover"), hovered, 0.13);
        let down = ctx.animate_bool_with_time(
            resp.id.with("down"),
            enabled && resp.is_pointer_button_down_on(),
            0.06,
        );
        let base = if enabled {
            Color32::from_rgba_unmultiplied(255, 255, 255, 24)
        } else {
            Color32::from_rgba_unmultiplied(255, 255, 255, 10)
        };
        let hover = Color32::from_rgba_unmultiplied(255, 96, 176, 62);
        let press = Color32::from_rgba_unmultiplied(255, 61, 146, 86);
        let fill = mix(base, mix(hover, press, down), t);
        let stroke = mix(
            mix(
                Color32::from_rgba_unmultiplied(255, 255, 255, 46),
                Color32::from_rgba_unmultiplied(255, 61, 146, 150),
                down * 0.5,
            ),
            Color32::from_rgba_unmultiplied(255, 61, 146, 210),
            t,
        );
        let text_color = if enabled { mix(TEXT, Color32::WHITE, t) } else { MUTED };
        let p = ui.painter();
        p.rect(
            rect,
            12.0,
            fill,
            egui::Stroke::new(1.0_f32, stroke),
            egui::StrokeKind::Inside,
        );
        p.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            display(17.0),
            text_color,
        );
        if enabled {
            resp.on_hover_cursor(egui::CursorIcon::PointingHand)
        } else {
            resp
        }
    }

    /// The big magenta action button, with a hover that eases instead of snapping.
    fn hero_button(
        ui: &mut egui::Ui,
        text: &str,
        width: f32,
        height: f32,
        enabled: bool,
    ) -> egui::Response {
        let (rect, resp) = ui
            .allocate_exact_size(egui::vec2(width, height), egui::Sense::click());
        let ctx = ui.ctx().clone();
        let h = ctx.animate_bool_with_time(
            resp.id.with("hover"),
            enabled && resp.hovered(),
            0.15,
        );
        let d = ctx.animate_bool_with_time(
            resp.id.with("down"),
            enabled && resp.is_pointer_button_down_on(),
            0.06,
        );
        let idle = if enabled { MAGENTA } else { Color32::from_rgb(96, 58, 82) };
        let fill = mix(mix(idle, Color32::from_rgb(255, 96, 176), h), Color32::from_rgb(206, 34, 116), d);
        let p = ui.painter();
        if h > 0.01 {
            // Soft halo that fades in under the pointer.
            p.rect(
                rect.expand(3.0 * h),
                17.0,
                Color32::from_rgba_unmultiplied(255, 61, 146, (26.0 * h) as u8),
                egui::Stroke::NONE,
                egui::StrokeKind::Outside,
            );
        }
        p.rect(
            rect,
            14.0,
            fill,
            egui::Stroke::new(
                1.0_f32,
                mix(Color32::from_rgb(255, 130, 190), Color32::WHITE, h * 0.7),
            ),
            egui::StrokeKind::Inside,
        );
        p.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            display(24.0),
            Color32::from_rgb(255, 250, 252),
        );
        if enabled {
            resp.on_hover_cursor(egui::CursorIcon::PointingHand)
        } else {
            resp
        }
    }

    fn tab_button(ui: &mut egui::Ui, text: &str, active: bool) -> egui::Response {
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(96.0, 34.0), egui::Sense::click());
        let ctx = ui.ctx().clone();
        let a = ctx.animate_bool_with_time(resp.id.with("on"), active, 0.16);
        let h = ctx.animate_bool_with_time(resp.id.with("hover"), resp.hovered(), 0.12);
        let idle = mix(
            Color32::from_rgba_unmultiplied(255, 255, 255, 14),
            Color32::from_rgba_unmultiplied(255, 255, 255, 30),
            h,
        );
        let fill = mix(idle, MAGENTA, a);
        let stroke = mix(
            mix(EDGE, Color32::from_rgba_unmultiplied(255, 61, 146, 140), h),
            MAGENTA,
            a,
        );
        let color = mix(mix(TEXT, Color32::WHITE, h), Color32::from_rgb(24, 10, 20), a);
        let p = ui.painter();
        p.rect(
            rect,
            11.0,
            fill,
            egui::Stroke::new(1.0_f32, stroke),
            egui::StrokeKind::Inside,
        );
        p.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            display(16.0),
            color,
        );
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    }

    /// One language shortcut in the rail. The active one is filled magenta so
    /// the current language is obvious without opening a dropdown.
    fn lang_pill(ui: &mut egui::Ui, code: &str, active: bool) -> egui::Response {
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(34.0, 27.0), egui::Sense::click());
        let ctx = ui.ctx().clone();
        let h = ctx.animate_bool_with_time(resp.id.with("hover"), resp.hovered(), 0.12);
        let idle = mix(
            Color32::from_rgba_unmultiplied(255, 255, 255, 12),
            Color32::from_rgba_unmultiplied(255, 255, 255, 28),
            h,
        );
        let fill = if active { MAGENTA } else { idle };
        ui.painter().rect(
            rect,
            8.0,
            fill,
            egui::Stroke::new(
                1.0_f32,
                if active {
                    MAGENTA
                } else {
                    mix(EDGE, Color32::from_rgba_unmultiplied(255, 61, 146, 130), h)
                },
            ),
            egui::StrokeKind::Inside,
        );
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            code,
            display(12.5),
            if active { Color32::from_rgb(24, 10, 20) } else { SOFT },
        );
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    }

    /// A rounded bar in the launcher palette. `frac` is 0..1.
    fn accent_bar(ui: &mut egui::Ui, width: f32, height: f32, frac: f32, color: Color32) {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
        let p = ui.painter();
        p.rect_filled(
            rect,
            height * 0.5,
            Color32::from_rgba_unmultiplied(255, 255, 255, 10),
        );
        let frac = frac.clamp(0.0, 1.0);
        if frac > 0.0 {
            let fill = egui::Rect::from_min_size(
                rect.min,
                egui::vec2((rect.width() * frac).max(height), height),
            );
            p.rect_filled(
                fill,
                height * 0.5,
                Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 225),
            );
        }
    }

    /// One side of the performance card: label, number and a small bar.
    /// `scale` is the shared ceiling for both bars, so their lengths keep the
    /// real ratio instead of flattening against a fixed 120 fps.
    fn fps_meter(
        ui: &mut egui::Ui,
        label: &str,
        value: Option<u32>,
        color: Color32,
        width: f32,
        scale: u32,
    ) {
        ui.vertical(|ui| {
            ui.set_width(width);
            ui.horizontal(|ui| {
                ui.label(disp(label, 12.0).color(MUTED));
                ui.add_space(8.0);
                match value {
                    Some(v) => ui.label(disp(format!("{v} fps"), 24.0).color(color)),
                    None => ui.label(disp("—", 24.0).color(MUTED)),
                };
            });
            ui.add_space(5.0);
            let frac = match value {
                Some(v) if scale > 0 => (v as f32 / scale as f32).clamp(0.0, 1.0),
                _ => 0.0,
            };
            accent_bar(ui, width, 8.0, frac, color);
        });
    }

    /// Five segments, one per captured reading.
    fn bench_progress(ui: &mut egui::Ui, width: f32, frac: f32, color: Color32) {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 6.0), egui::Sense::hover());
        let gap = 5.0;
        let seg = ((rect.width() - gap * (BENCH_STEPS as f32 - 1.0)) / BENCH_STEPS as f32).max(1.0);
        for i in 0..BENCH_STEPS {
            let r = egui::Rect::from_min_size(
                egui::pos2(rect.left() + i as f32 * (seg + gap), rect.top()),
                egui::vec2(seg, rect.height()),
            );
            let done = i as f32 + 1.0 <= frac * BENCH_STEPS as f32 + 0.001;
            ui.painter().rect_filled(
                r,
                3.0,
                if done {
                    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 225)
                } else {
                    Color32::from_rgba_unmultiplied(255, 255, 255, 12)
                },
            );
        }
    }

    pub fn gui(opts: Options) -> R<()> {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1120.0, 690.0])
                .with_min_inner_size([960.0, 600.0])
                .with_icon(app_icon())
                // Shown once the first frame is drawn, so the light background
                // Windows paints on a fresh window never flashes at the user.
                .with_visible(false),
            ..Default::default()
        };
        eframe::run_native(
            NAME,
            options,
            Box::new(move |cc| {
                load_fonts(&cc.egui_ctx);
                Ok(Box::new(Window::new(opts)))
            }),
        )
        .map_err(|e| format!("could not open the window: {e}"))
    }

    struct Window {
        opts: Options,
        lang: Lang,
        log: Vec<String>,
        busy: bool,
        status: String,
        rx: Option<Receiver<Msg>>,
        det: Detection,
        path: String,
        tab: Tab,
        show: Option<&'static str>,
        upd: Option<Update>,
        upd_rx: Option<Receiver<Option<Update>>>,
        dl_rx: Option<Receiver<UpMsg>>,
        progress: Arc<AtomicU64>,
        total: u64,
        dl_done: Option<PathBuf>,
        dl_err: Option<String>,
        starting_setup: bool,
        news_open: Vec<bool>,
        /// (lang, title, date, summary, body, version, art code)
        local_news: Vec<(String, String, String, String, String, String, usize)>,
        bench: Option<BenchRun>,
        bench_fps: Option<u32>,
        bench_java: Option<u32>,
        /// One flag per `GameVersion::ALL`, filled once so the rail can draw its
        /// ready/replacement dots without decoding five jars every frame.
        version_ready: Vec<bool>,
        /// Where the selected build and its level.dat live; "Open folder" shows
        /// this instead of the .minecraft textures folder.
        game_dir: PathBuf,
        /// 0..1, restarted whenever the tab changes so the view eases in.
        fade: f32,
        shown_tab: Tab,
        /// Kept while the panel fades out, so it does not vanish mid-animation.
        panel_text: &'static str,
        /// When the window opened, for the intro animation.
        t0: std::time::Instant,
        /// Set on the first drawn frame, when the window is allowed to show.
        revealed: bool,
    }

    impl Window {
        fn new(opts: Options) -> Self {
            let lang = opts.lang;
            let t = texts(lang);
            let det = detect_atlas(&opts);
            let status = det.summary(&t);
            let path = det.path.as_ref().map(|p| p.display().to_string()).unwrap_or_default();
            let which = std::env::var("RUSTIUM_PANEL").unwrap_or_default();
            let show = match which.as_str() {
                "how" | "instalar" => Some(t.how_text),
                "legal" => Some(t.legal),
                _ => None,
            };
            let tab = match which.as_str() {
                "news" | "novedades" => Tab::News,
                _ => Tab::Home,
            };

            let (utx, urx) = channel();
            std::thread::spawn(move || {
                let _ = utx.send(check_update());
            });

            let version_ready = GameVersion::ALL
                .iter()
                .map(|v| version_textures(&opts.dir_minecraft, *v))
                .collect();
            let game_dir = find_game(&t, opts.game)
                .ok()
                .and_then(|p| p.parent().map(Path::to_path_buf))
                .or_else(|| {
                    std::env::current_exe()
                        .ok()
                        .and_then(|p| p.parent().map(Path::to_path_buf))
                })
                .unwrap_or_default();

            Window {
                opts,
                lang,
                log: Vec::new(),
                busy: false,
                status,
                rx: None,
                det,
                path,
                tab,
                show,
                upd: None,
                upd_rx: Some(urx),
                dl_rx: None,
                progress: Arc::new(AtomicU64::new(0)),
                total: 0,
                dl_done: None,
                dl_err: None,
                starting_setup: false,
                // Debug hook: RUSTIUM_NEWS_OPEN starts with every card open so a
                // screenshot shows the expanded layout without clicking each one.
                news_open: if std::env::var("RUSTIUM_NEWS_OPEN").is_ok() {
                    vec![true; 16]
                } else {
                    Vec::new()
                },
                local_news: load_local_news(),
                bench: None,
                bench_fps: None,
                bench_java: None,
                version_ready,
                game_dir,
                fade: 0.0,
                shown_tab: Tab::Home,
                panel_text: "",
                t0: std::time::Instant::now(),
                revealed: false,
            }
        }

        /// Opening: black, the Rustium R drops in from above, bounces to a stop
        /// in the centre, then the letter and the black dissolve together.
        fn splash(&self, ctx: &egui::Context) {
            if std::env::var("RUSTIUM_NOSPLASH").is_ok() {
                return;
            }
            // The letter reaches the centre by DROP, bounces until HOLD and by
            // TOTAL the black and the letter have both gone.
            const DROP: f32 = 0.46;
            const HOLD: f32 = 1.70;
            const TOTAL: f32 = 2.15;
            let e = self.t0.elapsed().as_secs_f32();
            if e >= TOTAL {
                return;
            }
            ctx.request_repaint();
            let rect = ctx.screen_rect();
            let p = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("splash"),
            ));
            // One curve drives the black and the letter, so they leave together.
            let out = if e < HOLD {
                1.0
            } else {
                1.0 - smooth_step((e - HOLD) / (TOTAL - HOLD))
            };
            p.rect_filled(
                rect,
                0.0,
                Color32::from_rgba_unmultiplied(0, 0, 0, (255.0 * out).round() as u8),
            );
            // The letter keeps its size and falls in from above; after touching
            // the centre it rebounds a few times with a decaying arc.
            let size = 176.0;
            let appear = (e / 0.14).clamp(0.0, 1.0);
            let alpha = (255.0 * appear * out).round() as u8;
            if alpha == 0 {
                return;
            }
            let fall = (e / DROP).clamp(0.0, 1.0);
            let drop = fall * fall;
            let bounce = logo_bounce(e - DROP);
            let from = rect.height() * 0.55 + size;
            let center = rect.center() + egui::vec2(0.0, -from * (1.0 - drop) - bounce);
            let tint = |a: u8| {
                Color32::from_rgba_unmultiplied(MAGENTA.r(), MAGENTA.g(), MAGENTA.b(), a)
            };
            // A soft magenta halo that follows the letter down.
            for (i, base) in [9u8, 7, 5].iter().enumerate() {
                let a = (*base as f32 * appear * out).round() as u8;
                if a > 0 {
                    p.circle_filled(center, size * (0.86 - i as f32 * 0.16), tint(a));
                }
            }
            // Lay the glyph out with the faded colour: egui uses the galley's
            // own colour, so an opaque glyph would survive the fade and pop out.
            let g = p.layout_no_wrap("R".to_owned(), pixel(ctx, size), tint(alpha));
            p.galley(center - g.size() * 0.5, g, tint(alpha));
        }

        fn refresh(&mut self) {
            let t = texts(self.lang);
            self.det = detect_atlas(&self.opts);
            self.status = self.det.summary(&t);
            if let Some(p) = &self.det.path {
                self.path = p.display().to_string();
            }
            // A visible trace, so pressing Revisar always answers with something.
            let line = format!("⟳ {}", self.status);
            self.log.push(line);
        }

        fn use_path(&mut self) {
            let t = texts(self.lang);
            let typed = self.path.trim().to_string();
            if typed.is_empty() {
                self.opts.atlas = None;
                self.refresh();
                return;
            }
            let path = PathBuf::from(&typed);
            if atlas_ok(&path) {
                self.log.push(fill(&t.route_ok, "{path}", &typed));
                self.opts.atlas = Some(path);
            } else {
                self.log.push(fill(&t.route_bad, "{path}", &typed));
                self.opts.atlas = None;
            }
            self.refresh();
        }

        fn play(&mut self) {
            if self.busy {
                return;
            }
            let t = texts(self.lang);
            let (tx, rx) = channel();
            self.rx = Some(rx);
            self.busy = true;
            self.status = t.launching.to_string();
            let mut opts = self.opts.clone();
            opts.lang = self.lang;
            if opts.atlas.is_none() {
                opts.atlas = self.det.path.clone();
            }
            std::thread::spawn(move || {
                let line = tx.clone();
                set_report(Box::new(move |s: &str| {
                    let _ = line.send(Msg::Line(s.to_string()));
                }));
                let result = run(&opts);
                clear_report();
                let _ = tx.send(Msg::Done(result));
            });
        }

        fn start_update(&mut self) {
            let Some(u) = self.upd.clone() else { return };
            let (tx, rx) = channel();
            self.dl_rx = Some(rx);
            self.dl_done = None;
            self.dl_err = None;
            self.total = u.setup_size;
            self.progress.store(0, Ordering::Relaxed);
            let prog = self.progress.clone();
            let lang = self.lang;
            std::thread::spawn(move || match download_setup(&u, &prog, lang) {
                Ok(p) => {
                    let _ = tx.send(UpMsg::Ready(p));
                }
                Err(e) => {
                    let _ = tx.send(UpMsg::Failed(e));
                }
            });
        }

        fn pump(&mut self) {
            if let Some(rx) = self.upd_rx.take() {
                match rx.try_recv() {
                    Ok(found) => self.upd = found,
                    Err(std::sync::mpsc::TryRecvError::Empty) => self.upd_rx = Some(rx),
                    Err(_) => {}
                }
            }
            if let Some(rx) = self.dl_rx.take() {
                match rx.try_recv() {
                    Ok(UpMsg::Ready(p)) => {
                        self.dl_done = Some(p);
                    }
                    Ok(UpMsg::Failed(e)) => self.dl_err = Some(e),
                    Err(std::sync::mpsc::TryRecvError::Empty) => self.dl_rx = Some(rx),
                    Err(_) => {}
                }
            }
            if let Some(mut run) = self.bench.take() {
                let mut done = false;
                while let Ok(line) = run.rx.try_recv() {
                    if let Some(v) = line.strip_prefix("JAVA ") {
                        if let Ok(v) = v.trim().parse::<u32>() {
                            self.bench_java = Some(v);
                        }
                    }
                    if let Some(v) = line.split("fps=").nth(1) {
                        if let Ok(v) = v.trim().parse::<u32>() {
                            run.fps.push(v);
                        }
                    }
                    if line == "RUST DONE" {
                        run.rust_done = true;
                        self.bench_fps = run.settled();
                    }
                    // Only the thread's own END closes the run: the game also
                    // prints BENCH DONE when it stops, and Java still follows.
                    if line == "END" {
                        done = true;
                    }
                }
                if done {
                    self.bench_fps = run.settled();
                    beep();
                } else {
                    self.bench = Some(run);
                }
            }
            let mut finished = false;
            if let Some(rx) = self.rx.take() {
                loop {
                    match rx.try_recv() {
                        Ok(Msg::Line(l)) => {
                            if !l.trim().is_empty() {
                                self.log.push(l);
                            }
                        }
                        Ok(Msg::Done(r)) => {
                            let t = texts(self.lang);
                            self.busy = false;
                            finished = true;
                            self.status = match &r {
                                Ok(()) => t.done.to_string(),
                                Err(e) => format!("Error: {e}"),
                            };
                        }
                        Err(_) => break,
                    }
                }
                if !finished {
                    self.rx = Some(rx);
                }
            }
        }

        /// Runs the port at four angles and keeps the average frame rate.
        fn start_bench(&mut self) {
            let t = texts(self.lang);
            // The benchmark measures the rd-132211 port against the Java original,
            // so it always starts that build, whatever the picker says.
            let Ok(exe) = find_game(&t, GameVersion::Rd132211) else { return };
            let (tx, rx) = channel();
            self.bench = Some(BenchRun { rx, fps: Vec::new(), rust_done: false });
            self.bench_fps = None;
            self.bench_java = None;
            std::thread::spawn(move || {
                use std::io::BufRead;
                // The port goes first so the progress bar starts moving right
                // away; the Java original follows and both keep the same spot.
                let child = std::process::Command::new(&exe)
                    .env("RD_BENCH", "1")
                    // Same fraction the agent hands the original, so both spawn on
                    // the same block looking the same way.
                    .env("RD_SPAWN", BENCH_SPAWN.to_string())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::null())
                    .spawn();
                let Ok(mut child) = child else {
                    let _ = tx.send("RUST DONE".to_string());
                    let _ = tx.send("END".to_string());
                    return;
                };
                if let Some(out) = child.stdout.take() {
                    let lines = std::io::BufReader::new(out).lines().map_while(Result::ok);
                    for line in lines {
                        let _ = tx.send(line);
                    }
                }
                let _ = child.wait();
                let _ = tx.send("RUST DONE".to_string());
                // Then the player's own Java copy, measured on its own so the
                // two numbers are not distorted by sharing the GPU.
                if let Some(fps) = measure_java() {
                    let _ = tx.send(format!("JAVA {fps}"));
                }
                let _ = tx.send("END".to_string());
            });
        }

        fn sidebar(&mut self, ui: &mut egui::Ui) {
            let t = texts(self.lang);
            let u = ui_texts(self.lang);

            ui.add_space(4.0);
            pixel_text(ui, "RUSTIUM", 16.0, MAGENTA);
            ui.label(RichText::new("RubyDung RS").size(12.0).color(MUTED));
            ui.add_space(26.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new(u.sidebar_instances).size(10.5).color(MUTED).strong());
            });
            ui.add_space(8.0);

            // The rail is the version picker: one list, no second combo box
            // and no separate "new instance" dialog.
            let mut pick: Option<GameVersion> = None;
            for (i, v) in GameVersion::ALL.iter().enumerate() {
                let ok = self.version_ready.get(i).copied().unwrap_or(false);
                let (dot, sub) = if ok {
                    (GREEN, u.instance_ready)
                } else {
                    (AMBER, u.instance_replacement)
                };
                if instance_row(ui, v.id(), sub, dot, *v == self.opts.game).clicked() {
                    pick = Some(*v);
                }
                ui.add_space(8.0);
            }
            if let Some(v) = pick {
                self.opts.game = v;
                save_config(self.lang, v);
                if let Ok(p) = find_game(&t, v) {
                    if let Some(dir) = p.parent() {
                        self.game_dir = dir.to_path_buf();
                    }
                }
            }

            // Footer: reserved at the bottom so it never falls off a short window.
            ui.add_space((ui.available_height() - 112.0).max(6.0));
            if ui
                .add(
                    egui::Button::new(RichText::new(u.news_shortcut).size(12.5).color(SOFT))
                        .fill(Color32::TRANSPARENT)
                        .corner_radius(6),
                )
                .clicked()
            {
                self.tab = Tab::News;
            }
            if ui
                .add(
                    egui::Button::new(RichText::new(t.legal_title).size(12.5).color(SOFT))
                        .fill(Color32::TRANSPARENT)
                        .corner_radius(6),
                )
                .clicked()
            {
                self.show = if self.show == Some(t.legal) { None } else { Some(t.legal) };
            }
            ui.add_space(8.0);
            ui.label(RichText::new(u.language_label).size(10.5).color(MUTED));
            ui.add_space(3.0);
            let mut elegido = self.lang;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                for l in Lang::ALL {
                    let code = l.code().to_uppercase();
                    if lang_pill(ui, &code, l == self.lang)
                        .on_hover_text(l.name())
                        .clicked()
                    {
                        elegido = l;
                    }
                }
            });
            if elegido != self.lang {
                self.lang = elegido;
                save_config(elegido, self.opts.game);
                let t2 = texts(elegido);
                // Old messages were written in the old language: drop them.
                self.log.clear();
                self.status = self.det.summary(&t2);
                if let Some(p) = &self.det.path {
                    self.path = p.display().to_string();
                }
                if self.show == Some(t.legal) {
                    self.show = Some(t2.legal);
                }
            }
            ui.add_space(6.0);
            ui.label(
                RichText::new(format!("{} {VERSION}", u.version_label))
                    .size(10.5)
                    .color(MUTED),
            );
        }

        fn update_banner(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
            let u = ui_texts(self.lang);
            let Some(upd) = self.upd.clone() else { return };
            egui::Frame::NONE
                .fill(Color32::from_rgba_unmultiplied(255, 61, 146, 26))
                .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 61, 146, 90)))
                .corner_radius(16)
                .inner_margin(egui::Margin::symmetric(18, 14))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    if let Some(err) = &self.dl_err {
                        ui.label(RichText::new(err).size(12.5).color(AMBER));
                        ui.add_space(4.0);
                    }
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(fill(&u.update_available, "{version}", &upd.version))
                                .size(14.0)
                                .strong()
                                .color(BLUE),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if self.dl_done.is_some() {
                                ui.label(RichText::new(u.update_ready).size(12.0).color(GREEN));
                            } else if self.dl_rx.is_some() {
                                let done = self.progress.load(Ordering::Relaxed);
                                let total = if self.total > 0 { self.total } else { 1 };
                                let frac = (done as f32 / total as f32).clamp(0.0, 1.0);
                                let pct = (frac * 100.0).round() as u32;
                                ui.add(
                                    egui::ProgressBar::new(frac)
                                        .desired_width(240.0)
                                        .text(
                                            RichText::new(
                                                fill(&u.update_downloading, "{pct}", &pct.to_string()),
                                            )
                                            .size(11.5),
                                        ),
                                );
                            } else {
                                let go = egui::Button::new(
                                    RichText::new(fill(&u.update_now, "{version}", &upd.version))
                                        .size(13.0)
                                        .color(Color32::from_rgb(10, 18, 28)),
                                )
                                .fill(BLUE)
                                .corner_radius(8);
                                if ui.add(go).clicked() {
                                    self.start_update();
                                }
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new(u.update_page).size(11.5).color(SOFT),
                                        )
                                        .fill(Color32::TRANSPARENT),
                                    )
                                    .clicked()
                                {
                                    open_in_browser(&upd.url);
                                }
                            }
                        });
                    });
                });

            // Once the installer is on disk, launch it and step aside so it can
            // replace the executables.
            if self.dl_done.is_some() && !self.starting_setup {
                self.starting_setup = true;
                if let Some(p) = &self.dl_done {
                    let _ = std::process::Command::new(p).spawn();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
    }

    impl eframe::App for Window {
        /// The window is black from its very first frame, so not a pixel of the
        /// launcher can flash before the opening curtain is painted.
        fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
            [0.0, 0.0, 0.0, 1.0]
        }

        fn update(&mut self, ctx: &egui::Context, _f: &mut eframe::Frame) {
            ctx.set_visuals(egui::Visuals::dark());
            // The window is created hidden while fonts and the GPU wake up. The
            // intro clock starts with the first paint, not with construction, so
            // the fall is never half over before the window is on screen.
            if !self.revealed {
                self.t0 = std::time::Instant::now();
            }
            // Debug hook: start the measurement as soon as the window opens.
            if std::env::var("RUSTIUM_BENCH").is_ok()
                && self.bench.is_none()
                && self.bench_fps.is_none()
            {
                self.start_bench();
            }
            self.pump();
            // Ease the view in after a tab change instead of snapping.
            let dt = ctx.input(|i| i.stable_dt).min(0.1);
            if self.shown_tab != self.tab {
                self.shown_tab = self.tab;
                self.fade = 0.0;
            }
            if self.fade < 1.0 {
                self.fade = (self.fade + dt / 0.18).min(1.0);
                ctx.request_repaint();
            }
            if self.busy || self.upd_rx.is_some() || self.dl_rx.is_some() || self.bench.is_some()
            {
                ctx.request_repaint_after(std::time::Duration::from_millis(120));
            }
            let t = texts(self.lang);
            let u = ui_texts(self.lang);
            let (l_ver, l_tex, l_perf, more_news) = extra_texts(self.lang);
            let busy = self.busy;
            // The texture status follows the build picked in the rail. Only
            // rd-132211 takes a hand-picked atlas; the legacy builds read their
            // own terrain.png from their jar.
            let slot = GameVersion::ALL
                .iter()
                .position(|v| *v == self.opts.game)
                .unwrap_or(0);
            let have = if self.opts.game == GameVersion::Rd132211 {
                self.det.path.is_some()
            } else {
                self.version_ready.get(slot).copied().unwrap_or(false)
            };
            let detected = self.det.origin == "detected";
            let tab = self.tab;

            egui::SidePanel::left("rail")
                .exact_width(250.0)
                .resizable(false)
                .frame(
                    egui::Frame::NONE
                        .fill(RAIL)
                        .inner_margin(egui::Margin { left: 20, right: 20, top: 20, bottom: 16 }),
                )
                .show(ctx, |ui| self.sidebar(ui));

            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ctx, |ui| {
                    backdrop(ui);
                    ui.multiply_opacity(0.25 + 0.75 * self.fade);
                    egui::Frame::NONE
                        .inner_margin(egui::Margin { left: 30, right: 30, top: 22, bottom: 16 })
                        .show(ui, |ui| {
                            // ---- tabs
                            ui.horizontal(|ui| {
                                if tab_button(ui, u.tab_home, tab == Tab::Home).clicked() {
                                    self.tab = Tab::Home;
                                    // Leaving the tab closes whatever panel was open,
                                    // so nothing follows the user around.
                                    self.show = None;
                                }
                                if tab_button(ui, u.tab_news, tab == Tab::News).clicked() {
                                    self.tab = Tab::News;
                                    self.show = None;
                                }
                            });
                            ui.add_space(16.0);

                            ui.label(disp("Rustium", 54.0).color(Color32::from_rgb(250, 244, 250)));
                            // The long path is only worth showing when something is
                            // missing; otherwise it lives inside the texture card.
                            if !have {
                                ui.label(RichText::new(&self.status).size(13.0).color(SOFT));
                            }
                            ui.add_space(14.0);

                            self.update_banner(ui, ctx);

                            if tab == Tab::Home {
                                ui.add_space(2.0);
                                // The version lives in the rail (one list), so the
                                // home tab goes straight to playing.
                                ui.horizontal(|ui| {
                                    if hero_button(ui, t.play, 210.0, 54.0, !busy).clicked() {
                                        self.play();
                                    }
                                    if plain_button(ui, t.refresh, 120.0, !busy).clicked() {
                                        self.refresh();
                                    }
                                    if plain_button(ui, u.folder, 150.0, !self.game_dir.as_os_str().is_empty()).clicked() {
                                        reveal_dir(&self.game_dir);
                                    }
                                    if plain_button(ui, t.how, 160.0, !busy).clicked() {
                                        self.show = if self.show == Some(t.how_text) {
                                            None
                                        } else {
                                            Some(t.how_text)
                                        };
                                    }
                                    if busy {
                                        ui.spinner();
                                    }
                                });

                                ui.add_space(18.0);
                                card(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    let (title, color) = if have {
                                        (t.found_title, GREEN)
                                    } else {
                                        (t.missing_title, AMBER)
                                    };
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(title).size(12.5).strong().color(color));
                                        ui.add_space(6.0);
                                        let manual = self.opts.game == GameVersion::Rd132211 && !detected;
                                        let sub = if have {
                                            if manual { t.manual_src } else { t.found_src }.to_string()
                                        } else if self.opts.game == GameVersion::Rd132211 {
                                            t.missing_src.to_string()
                                        } else {
                                            fill(&t.tex_none, "{version}", self.opts.game.id())
                                                .trim()
                                                .to_string()
                                        };
                                        ui.label(RichText::new(&sub).size(12.5).color(SOFT));
                                    });
                                    if !have && self.opts.game == GameVersion::Rd132211 {
                                        ui.add_space(4.0);
                                        ui.label(RichText::new(t.missing_hint).size(12.5).color(SOFT));
                                    }
                                    if self.opts.game == GameVersion::Rd132211 {
                                        ui.add_space(10.0);
                                        ui.label(RichText::new(t.path_label).size(11.0).color(MUTED));
                                        ui.add_space(4.0);
                                        ui.horizontal(|ui| {
                                            let w = (ui.available_width() - 150.0).max(200.0);
                                            egui::Frame::NONE
                                                .fill(Color32::from_rgba_premultiplied(14, 14, 14, 14))
                                                .corner_radius(10)
                                                .inner_margin(egui::Margin::symmetric(12, 9))
                                                .show(ui, |ui| {
                                                    ui.add_sized(
                                                        [w - 24.0, 18.0],
                                                        egui::TextEdit::singleline(&mut self.path)
                                                            .frame(false)
                                                            .background_color(Color32::TRANSPARENT)
                                                            .text_color(SOFT)
                                                            .hint_text(r"C:\...\.minecraft\versions\rd-132211\rd-132211.jar"),
                                                    );
                                                });
                                            let use_btn = egui::Button::new(
                                                RichText::new(t.use_path).size(13.0).color(TEXT),
                                            )
                                            .fill(GLASS_HI)
                                            .corner_radius(10)
                                            .min_size(egui::vec2(138.0, 38.0));
                                            if ui.add(use_btn).clicked() {
                                                self.use_path();
                                            }
                                        });
                                    }
                                });

                                ui.add_space(12.0);
                                ui.horizontal(|ui| {
                                    stat_card(ui, l_ver, self.opts.game.id(), TEXT, 140.0);
                                    ui.add_space(10.0);
                                    stat_card(
                                        ui,
                                        l_tex,
                                        if have { "Original" } else { "Custom" },
                                        if have { GREEN } else { AMBER },
                                        170.0,
                                    );
                                });

                                ui.add_space(10.0);
                                // Rendimiento: dos medidores con la misma barra
                                // redondeada del resto del launcher.
                                {
                                    egui::Frame::NONE
                                        .fill(GLASS)
                                        .stroke(egui::Stroke::new(1.0_f32, EDGE))
                                        .corner_radius(14)
                                        .inner_margin(egui::Margin::symmetric(16, 12))
                                        .show(ui, |ui| {
                                            ui.set_width((ui.available_width() - 32.0).max(320.0));
                                            let running = self.bench.is_some();
                                            let label = match &self.bench {
                                                Some(run)
                                                    if run.rust_done && self.bench_java.is_none() =>
                                                {
                                                    u.bench_java_wait.to_string()
                                                }
                                                Some(run) => fill(
                                                    &fill(
                                                        &u.bench_running,
                                                        "{step}",
                                                        &run.fps.len().to_string(),
                                                    ),
                                                    "{total}",
                                                    &BENCH_STEPS.to_string(),
                                                ),
                                                None => u.measure_fps.to_string(),
                                            };
                                            ui.horizontal(|ui| {
                                                ui.label(disp(l_perf, 11.5).color(MUTED));
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(
                                                        egui::Align::Center,
                                                    ),
                                                    |ui| {
                                                        if plain_button(
                                                            ui,
                                                            &label,
                                                            150.0,
                                                            !running,
                                                        )
                                                        .clicked()
                                                        {
                                                            self.start_bench();
                                                        }
                                                    },
                                                );
                                            });
                                            ui.add_space(9.0);
                                            let meter_w =
                                                ((ui.available_width() - 22.0) * 0.5).max(150.0);
                                            // Both bars share the bigger reading, rounded up
                                            // to a round number: 582 and 105 stay apart.
                                            let scale = self
                                                .bench_java
                                                .max(self.bench_fps)
                                                .unwrap_or(0)
                                                .div_ceil(50)
                                                * 50;
                                            ui.horizontal(|ui| {
                                                fps_meter(
                                                    ui,
                                                    "Java",
                                                    self.bench_java,
                                                    BLUE,
                                                    meter_w,
                                                    scale,
                                                );
                                                ui.add_space(22.0);
                                                fps_meter(
                                                    ui,
                                                    "Rust",
                                                    self.bench_fps,
                                                    GREEN,
                                                    meter_w,
                                                    scale,
                                                );
                                            });
                                            if let Some(run) = &self.bench {
                                                ui.add_space(10.0);
                                                let frac = if run.rust_done
                                                    && self.bench_java.is_none()
                                                {
                                                    1.0
                                                } else {
                                                    run.fps.len() as f32
                                                        / BENCH_STEPS as f32
                                                };
                                                bench_progress(
                                                    ui,
                                                    ui.available_width(),
                                                    frac,
                                                    MAGENTA,
                                                );
                                            }
                                        });
                                }

                            } else {
                                // The cards grow when opened, so the list scrolls
                                // instead of pushing the footer off the window.
                                let h = (ui.available_height() - 46.0).max(140.0);
                                egui::ScrollArea::vertical()
                                    .max_height(h)
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        ui.add_space(2.0);
                                        ui.label(
                                            RichText::new(u.news_title)
                                                .size(14.0)
                                                .strong()
                                                .color(SOFT),
                                        );
                                        ui.add_space(10.0);
                                        // Intro cards, then the local extras from
                                        // news.txt, then the version history. The
                                        // list can grow at runtime, so the open
                                        // flags are resized after it is built.
                                        // (title, date, summary, body, version tag, art)
                                        let mut cards: Vec<(String, String, String, String, String, Art)> = vec![
                                            (
                                                u.n1_title.to_string(),
                                                u.n1_date.to_string(),
                                                u.n1_short.to_string(),
                                                u.n1_body.to_string(),
                                                format!("v{VERSION}"),
                                                Art::Launcher,
                                            ),
                                            (
                                                u.n2_title.to_string(),
                                                u.n2_date.to_string(),
                                                u.n2_short.to_string(),
                                                u.n2_body.to_string(),
                                                "rd-132211".to_string(),
                                                Art::First,
                                            ),
                                        ];
                                        for (lang, title, date, short, body, version, art) in &self.local_news {
                                            if lang != self.lang.code() {
                                                continue;
                                            }
                                            cards.push((
                                                title.clone(),
                                                date.clone(),
                                                short.clone(),
                                                body.clone(),
                                                version.clone(),
                                                match *art {
                                                    1 => Art::Bush,
                                                    2 => Art::Zombies,
                                                    3 => Art::Upright,
                                                    4 => Art::First,
                                                    _ => Art::World,
                                                },
                                            ));
                                        }
                                        // The build each version card talks about.
                                        let tags = [
                                            "rd-132211 … rd-161348",
                                            "rd-20090515",
                                            "rd-132328",
                                            "rd-20090515 · rd-161348",
                                            "rd-160052 · rd-20090515 · rd-161348",
                                            "rd-160052 · rd-20090515 · rd-161348",
                                            "rd-132328 · rd-20090515 · rd-160052 · rd-161348",
                                        ];
                                        for (i, item) in version_news(self.lang).iter().enumerate() {
                                            cards.push((
                                                item.0.to_string(),
                                                item.1.to_string(),
                                                item.2.to_string(),
                                                item.3.to_string(),
                                                tags.get(i).copied().unwrap_or("").to_string(),
                                                match i {
                                                    1 => Art::Bush,
                                                    2 => Art::Zombies,
                                                    3 => Art::Upright,
                                                    4 => Art::Ticks,
                                                    5 => Art::Growth,
                                                    6 => Art::Zombies,
                                                    _ => Art::World,
                                                },
                                            ));
                                        }
                                        if self.news_open.len() != cards.len() {
                                            self.news_open.resize(cards.len(), false);
                                        }
                                        for (i, (title, date, short, body, version, art)) in cards.iter().enumerate() {
                                            if i > 0 {
                                                ui.add_space(12.0);
                                            }
                                            news_card(
                                                ui,
                                                i,
                                                title,
                                                date,
                                                short,
                                                body,
                                                version,
                                                &mut self.news_open[i],
                                                u.see_more,
                                                u.see_less,
                                                *art,
                                            );
                                        }
                                        ui.add_space(12.0);
                                        ui.label(RichText::new(more_news).size(12.0).color(MUTED));
                                    });
                            }

                            if let Some(txt) = self.show {
                                self.panel_text = txt;
                            }
                            let panel = ctx.animate_value_with_time(
                                egui::Id::new("panel-fade"),
                                if self.show.is_some() { 1.0 } else { 0.0 },
                                0.18,
                            );
                            if panel > 0.01 {
                                ui.add_space(14.0);
                                let txt = self.panel_text;
                                ui.scope(|ui| {
                                    ui.multiply_opacity(panel);
                                    card(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        egui::ScrollArea::vertical()
                                            .max_height(200.0)
                                            .auto_shrink([false, false])
                                            .show(ui, |ui| {
                                                ui.label(
                                                    RichText::new(txt).size(13.0).color(TEXT),
                                                );
                                            });
                                    });
                                });
                            }

                            if !self.log.is_empty() {
                                ui.add_space(12.0);
                                egui::Frame::NONE
                                    .fill(Color32::from_rgba_premultiplied(12, 12, 12, 12))
                                    .corner_radius(10)
                                    .inner_margin(egui::Margin::symmetric(12, 9))
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        egui::ScrollArea::vertical()
                                            .max_height(96.0)
                                            .stick_to_bottom(true)
                                            .auto_shrink([false, false])
                                            .show(ui, |ui| {
                                                for l in &self.log {
                                                    ui.monospace(RichText::new(l).size(11.5).color(SOFT));
                                                }
                                            });
                                    });
                            }

                            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                                ui.label(RichText::new(t.legal_footer).size(11.0).color(MUTED));
                            });
                        });
                });
            self.splash(ctx);
            // The window was created hidden; show it now that this frame (black
            // curtain and all) is on its way to the screen.
            if !self.revealed {
                self.revealed = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.request_repaint();
            }
        }
    }

fn stat_card(ui: &mut egui::Ui, label: &str, value: &str, color: egui::Color32, width: f32) {
        egui::Frame::NONE
            .fill(GLASS)
            .stroke(egui::Stroke::new(1.0_f32, EDGE))
            .corner_radius(14)
            .inner_margin(egui::Margin::symmetric(16, 12))
            .show(ui, |ui| {
                ui.set_width(width);
                ui.label(disp(label, 11.5).color(MUTED));
                ui.add_space(3.0);
                ui.label(disp(value, 21.0).color(color));
            });
    }

    /// The picture used by the news card: the Rustium icon for launcher news
    /// and real captures from the port for the version news. Stored as raw RGB
    /// (or painted) so the launcher needs no decoder.
    #[derive(Clone, Copy)]
    enum Art {
        Launcher,
        World,
        Bush,
        Zombies,
        /// The upright bushes of rd-20090515 / rd-161348.
        Upright,
        /// The flat world of the very first version, rd-132211.
        First,
        /// The legacy world ticking at the original 20 Hz.
        Ticks,
        /// Grass and bushes updating after a level tick.
        Growth,
    }

    fn news_shot(
        ctx: &egui::Context,
        name: &str,
        bytes: &'static [u8],
    ) -> egui::TextureHandle {
        let id = egui::Id::new(("news-shot", name));
        if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
            return t;
        }
        let w = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        let h = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
        let img = egui::ColorImage::from_rgb([w, h], &bytes[8..8 + w * h * 3]);
        let tex = ctx.load_texture(name, img, egui::TextureOptions::LINEAR);
        ctx.data_mut(|d| d.insert_temp(id, tex.clone()));
        tex
    }

    fn art_thumb(ui: &mut egui::Ui, art: Art, w: f32, h: f32) {
        if !matches!(art, Art::Launcher) {
            let (name, bytes): (&str, &'static [u8]) = match art {
                Art::World => ("news-world", include_bytes!("../../assets/news_game.rgb")),
                Art::Bush => ("news-bush", include_bytes!("../../assets/news_bush.rgb")),
                Art::Zombies => ("news-zombies", include_bytes!("../../assets/news_zombies.rgb")),
                Art::Upright => ("news-upright", include_bytes!("../../assets/news_upright.rgb")),
                Art::First => ("news-first", include_bytes!("../../assets/news_first.rgb")),
                Art::Ticks => ("news-ticks", include_bytes!("../../assets/news_ticks.rgb")),
                Art::Growth => ("news-growth", include_bytes!("../../assets/news_growth.rgb")),
                Art::Launcher => unreachable!(),
            };
            let shot = news_shot(ui.ctx(), name, bytes);
            ui.add(
                egui::Image::from_texture(egui::load::SizedTexture::new(
                    shot.id(),
                    egui::vec2(w, h),
                ))
                .fit_to_exact_size(egui::vec2(w, h))
                .corner_radius(egui::CornerRadius::same(12)),
            );
            return;
        }
        // La tarjeta de Rustium vuelve a llevar su icono (la ventana con la R),
        // como antes: las capturas del juego son solo para las novedades de
        // versiones.
        let (rect, _) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
        let p = ui.painter().with_clip_rect(rect);
        p.rect_filled(rect, 12.0, Color32::from_rgb(58, 16, 44));
        for (i, a) in [12u8, 9, 6].iter().enumerate() {
            p.circle_filled(
                rect.left_top() + egui::vec2(w * 0.26, h * 0.22),
                w * (0.36 - i as f32 * 0.09),
                Color32::from_rgba_unmultiplied(MAGENTA.r(), MAGENTA.g(), MAGENTA.b(), *a),
            );
        }
        let win = egui::Rect::from_center_size(
            rect.center() + egui::vec2(0.0, 6.0),
            egui::vec2(w * 0.60, h * 0.58),
        );
        p.rect_filled(win, 8.0, Color32::from_rgb(26, 14, 28));
        for (i, c) in [MAGENTA, AMBER, GREEN].iter().enumerate() {
            p.circle_filled(win.left_top() + egui::vec2(11.0 + i as f32 * 9.5, 9.0), 2.6, *c);
        }
        let c = win.center() + egui::vec2(0.0, 8.0);
        let g = p.layout_no_wrap("R".to_owned(), pixel(ui.ctx(), h * 0.28), MAGENTA);
        p.galley(c - g.size() * 0.5, g, MAGENTA);
    }

    #[allow(clippy::too_many_arguments)]
    fn news_card(
        ui: &mut egui::Ui,
        card: usize,
        title: &str,
        date: &str,
        short: &str,
        full: &str,
        version: &str,
        open: &mut bool,
        more: &str,
        less: &str,
        art: Art,
    ) {
        egui::Frame::NONE
            .fill(GLASS)
            .stroke(egui::Stroke::new(1.0_f32, EDGE))
            .corner_radius(16)
            .inner_margin(egui::Margin::symmetric(16, 16))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                // The picture and the margins are already spoken for; the text
                // column must use what is left, or a long body widens the card
                // past the window and the right edge gets clipped.
                let text_w = (ui.available_width() - 176.0 - 16.0 - 8.0).max(220.0);
                ui.horizontal_top(|ui| {
                    art_thumb(ui, art, 176.0, 108.0);
                    ui.add_space(16.0);
                    ui.vertical(|ui| {
                        ui.set_max_width(text_w);
                        ui.horizontal(|ui| {
                            ui.label(disp(title, 19.0).color(TEXT));
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(RichText::new(date).size(11.5).color(MUTED));
                                    if !version.is_empty() {
                                        egui::Frame::NONE
                                            .fill(GLASS_HI)
                                            .corner_radius(6)
                                            .inner_margin(egui::Margin::symmetric(6, 2))
                                            .show(ui, |ui| {
                                                ui.label(
                                                    RichText::new(version)
                                                        .size(10.5)
                                                        .strong()
                                                        .color(MAGENTA),
                                                );
                                            });
                                    }
                                },
                            );
                        });
                        ui.add_space(7.0);
                        // The body grows instead of jumping and its text fades
                        // in with it. The index (not the title) is the id, so two
                        // cards with the same headline never share an animation
                        // and freeze it.
                        let t = ui.ctx().animate_bool_with_time(
                            egui::Id::new(("news-open", card)),
                            *open,
                            0.30,
                        );
                        // Keep the frames coming while the box is still moving,
                        // otherwise the animation freezes half way.
                        if (*open && t < 0.999) || (!*open && t > 0.001) {
                            ui.ctx().request_repaint();
                        }
                        // Spring only while opening; closing eases out evenly so
                        // it never snaps at the end.
                        let e = if *open { ease_out_back(t) } else { smooth_step(t) };
                        let fade = smooth_step(t);
                        let w = text_w;
                        let body = egui::FontId::proportional(13.0);
                        let gs = ui.painter().layout(short.to_owned(), body.clone(), SOFT, w);
                        let gf = ui.painter().layout(full.to_owned(), body, SOFT, w);
                        let h = egui::lerp(gs.size().y..=gf.size().y, e)
                            .clamp(0.0, gf.size().y * 1.12);
                        let (body_rect, _) =
                            ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
                        // One text at a time, revealed by the growing box and
                        // fading in as it opens: the two strings never overlap.
                        let shown = if *open { gf } else { gs };
                        let a = if *open { fade } else { 1.0 };
                        if a > 0.01 {
                            let alpha = (a.clamp(0.0, 1.0) * 255.0).round() as u8;
                            ui.painter()
                                .with_clip_rect(body_rect)
                                .galley(
                                    body_rect.left_top(),
                                    shown,
                                    Color32::from_rgba_unmultiplied(
                                        SOFT.r(),
                                        SOFT.g(),
                                        SOFT.b(),
                                        alpha,
                                    ),
                                );
                        }
                        ui.add_space(2.0);
                        let label = if *open { less } else { more };
                        if ui
                            .add(
                                egui::Button::new(disp(label, 14.0).color(MAGENTA))
                                    .frame(false),
                            )
                            .clicked()
                        {
                            *open = !*open;
                        }
                    });
                });
            });
    }

    /// A clickable rail card; the whole panel reacts, like a list row.
    fn instance_row(
        ui: &mut egui::Ui,
        name: &str,
        sub: &str,
        dot: Color32,
        selected: bool,
    ) -> egui::Response {
        let border = if selected {
            Color32::from_rgba_unmultiplied(255, 61, 146, 150)
        } else {
            EDGE
        };
        let inner = egui::Frame::NONE
            .fill(if selected { SELECTED } else { GLASS })
            .stroke(egui::Stroke::new(1.0_f32, border))
            .corner_radius(14)
            .inner_margin(egui::Margin::symmetric(14, 11))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(name).size(13.5).strong().color(TEXT));
                ui.add_space(3.0);
                ui.horizontal(|ui| {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(9.0, 9.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 4.0, dot);
                    ui.add_space(2.0);
                    ui.label(RichText::new(sub).size(11.5).color(SOFT));
                });
            });
        if selected {
            let r = inner.response.rect;
            let accent = egui::Rect::from_min_size(
                egui::pos2(r.left() + 1.0, r.top() + 13.0),
                egui::vec2(3.0, (r.height() - 26.0).max(4.0)),
            );
            ui.painter().rect_filled(accent, 2.0, MAGENTA);
        }
        let resp = ui.interact(
            inner.response.rect,
            ui.id().with(("instance", name)),
            egui::Sense::click(),
        );
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        resp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(data: &[u8]) -> String {
        let mut h = sha256::Sha256::new();
        h.update(data);
        h.hex()
    }

    #[test]
    fn sha256_matches_the_published_vectors() {
        assert_eq!(
            hash(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            hash(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let two_blocks = b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
        assert_eq!(
            hash(two_blocks),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
        // Coming in pieces has to give the same digest as in one go.
        let mut piece = sha256::Sha256::new();
        for chunk in two_blocks.chunks(7) {
            piece.update(chunk);
        }
        assert_eq!(
            piece.hex(),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn installers_must_come_over_https() {
        assert!(https_ok("https://github.com/x/y/releases/download/v1/setup.exe"));
        assert!(https_ok("http://127.0.0.1:8000/setup.exe"));
        assert!(https_ok("http://localhost:8099/setup.exe"));
        assert!(https_ok("http://[::1]:8099/setup.exe"));
        assert!(!https_ok("http://example.com/setup.exe"));
        assert!(!https_ok("javascript:alert(1)"));
        assert!(!https_ok("https://example.com/a b.exe"));
        assert!(!https_ok(""));
    }

    #[test]
    fn a_name_that_only_starts_like_loopback_is_not_this_machine() {
        assert!(!is_loopback("localhost.example.com/setup.exe"));
        assert!(!is_loopback("127.0.0.1.example.com/setup.exe"));
        assert!(!is_loopback("[::1]example.com/setup.exe"));
        assert!(!is_loopback("127.0.0.1:99999/setup.exe"));
        assert!(!is_loopback("localhost.attacker:8099/x.exe"));
        assert!(is_loopback("127.0.0.1"));
        assert!(is_loopback("LOCALHOST:8000/x"));
        assert!(is_loopback("user:pw@[::1]:8099/x"));
    }

    #[test]
    fn release_keeps_the_digest_and_drops_plain_http() {
        let body = r#"{"tag_name":"v0.2.0","html_url":"https://github.com/DandiMC/Rustium/releases/tag/v0.2.0",
        "assets":[{"browser_download_url":"https://github.com/DandiMC/Rustium/releases/download/v0.2.0/setup.exe",
        "size":1037,"digest":"sha256:9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"}]}"#;
        let u = parse_release(body).expect("release");
        assert_eq!(u.version, "v0.2.0");
        assert_eq!(u.setup_size, 1037);
        assert_eq!(
            u.setup_sha256.as_deref(),
            Some("9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08")
        );
        let plain = body.replace("https://github.com/DandiMC", "http://github.com/DandiMC");
        assert!(parse_release(&plain).expect("release").setup_url.is_none());
    }

    #[test]
    fn the_installer_comes_from_its_own_asset() {
        // The notes file is published first and wrongly lists a digest before its
        // url: the installer has to be read from its own object, `.exe` first.
        let body = r#"{"tag_name":"v0.3.0","assets":[
            {"digest":"sha256:1111111111111111111111111111111111111111111111111111111111111111",
             "name":"notes.txt","browser_download_url":"https://x/notes.txt","size":10},
            {"name":"Rustium-0.3.0-setup.exe","size":4242,"digest":"sha256:2222222222222222222222222222222222222222222222222222222222222222",
             "browser_download_url":"https://x/Rustium-0.3.0-setup.exe"}]}"#;
        let u = parse_release(body).expect("release");
        assert_eq!(u.setup_url.as_deref(), Some("https://x/Rustium-0.3.0-setup.exe"));
        assert_eq!(u.setup_size, 4242);
        assert_eq!(
            u.setup_sha256.as_deref(),
            Some("2222222222222222222222222222222222222222222222222222222222222222")
        );
        // The notes file alone is still a candidate, so the banner can say why it
        // will not run it.
        let notes_only = r#"{"tag_name":"v0.3.0","assets":[
            {"name":"notes.txt","browser_download_url":"https://x/notes.txt","size":10}]}"#;
        let u = parse_release(notes_only).expect("release");
        assert_eq!(u.setup_url.as_deref(), Some("https://x/notes.txt"));
        assert!(u.setup_sha256.is_none());
    }

    #[test]
    fn github_prints_the_digest_before_the_url() {
        // Field order copied from a live release: `size` and `digest` come before
        // `browser_download_url`, so reading forward from the url finds neither.
        let body = r#"{"tag_name":"v0.1.2","html_url":"https://github.com/DandiMC/Rustium/releases/tag/v0.1.2",
        "assets":[{"url":"https://api.github.com/repos/DandiMC/Rustium/releases/assets/1",
        "name":"Rustium-0.1.2-setup.exe","content_type":"application/octet-stream",
        "state":"uploaded","size":10416774,
        "digest":"sha256:e4b6dfc935c6911a32c6379790a78240b5ff18b6067ee6f39363ffc8c5947726",
        "download_count":0,"created_at":"2026-11-18T16:12:27Z",
        "browser_download_url":"https://github.com/DandiMC/Rustium/releases/download/v0.1.2/Rustium-0.1.2-setup.exe"}],
        "tarball_url":"https://api.github.com/repos/DandiMC/Rustium/tarball/v0.1.2",
        "body":"{\"note\":\"a brace } and a quote \\\" inside\"}"}"#;
        let u = parse_release(body).expect("release");
        assert_eq!(
            u.setup_url.as_deref(),
            Some("https://github.com/DandiMC/Rustium/releases/download/v0.1.2/Rustium-0.1.2-setup.exe")
        );
        assert_eq!(u.setup_size, 10416774);
        assert_eq!(
            u.setup_sha256.as_deref(),
            Some("e4b6dfc935c6911a32c6379790a78240b5ff18b6067ee6f39363ffc8c5947726")
        );
        assert!(is_newer(&u.version, "0.1.0"));
    }
}

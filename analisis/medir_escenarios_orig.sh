#!/bin/bash
# Mide FPS del original de Java en los mismos escenarios. El original imprime
# "N fps, M" una vez por segundo, sin marca de tiempo: la linea k es el segundo k.
# Raiz del proyecto: la carpeta que contiene a este script (analisis/).
PROJ="$(cd "$(dirname "$0")/.." && pwd)"
MC="$APPDATA/.minecraft"; LC="$MC/libraries"
CP="$(cygpath -w "$LC/org/lwjgl/lwjgl/lwjgl/2.9.1/lwjgl-2.9.1.jar");$(cygpath -w "$LC/org/lwjgl/lwjgl/lwjgl_util/2.9.1/lwjgl_util-2.9.1.jar");$(cygpath -w "$LC/net/java/jinput/jinput/2.0.5/jinput-2.0.5.jar")"
JAR="$(cygpath -w "$MC/versions/rd-132211/rd-132211.jar")"
BASE="$PROJ/analisis/mcmod/fps132211/escenarios_orig"
mkdir -p "$BASE"

matar() { powershell -NoProfile -Command "Get-Process -Name java -ErrorAction SilentlyContinue | Where-Object {\$_.MainWindowTitle -eq 'Game'} | Stop-Process -Force" 2>/dev/null; sleep 1; }

# $1 nombre $2 props java $3 segundos totales
escenario() {
  local nombre="$1" props="$2" total="$3"
  matar
  mkdir -p "$BASE/$nombre"; cd "$BASE/$nombre" || exit 1
  rm -f level.dat orig.log bloques_orig.raw luces_orig.raw
  java $props -Djava.library.path="$(cygpath -w "$PROJ/analisis/mcmod/natives")" \
    -cp "$(cygpath -w "$PROJ/analisis/orig_build/classes_cam132211");$JAR;$CP" \
    com.mojang.rubydung.RubyDung > orig.log 2>&1 &
  sleep "$total"
  matar
  echo "  $nombre: $(grep -ac 'fps,' orig.log) lineas"
}

escenario aparecer  ""                                                         6
escenario cielo     "-Dcareo.x=128.5 -Dcareo.y=43.9 -Dcareo.z=128.5 -Dcareo.pitch=85"  6
escenario suelo     "-Dcareo.x=128.5 -Dcareo.y=43.9 -Dcareo.z=128.5 -Dcareo.pitch=-85" 6
escenario vacio     "-Dcareo.x=400 -Dcareo.y=43.9 -Dcareo.z=128.5"              6
echo "escenarios del original hechos"

#!/bin/bash
# Mide FPS del port en distintas situaciones. Cada escenario lanza el juego en su
# carpeta (ahi escribe input_log.txt) y luego se queda con las muestras del tramo.
# Raiz del proyecto: la carpeta que contiene a este script (analisis/).
PROJ="$(cd "$(dirname "$0")/.." && pwd)"
EXE="$PROJ/target/x86_64-pc-windows-gnu/release/rubydung-rs.exe"
BASE="$PROJ/analisis/mcmod/fps132211/escenarios"
mkdir -p "$BASE"

matar() { powershell -NoProfile -Command "Get-Process -Name rubydung-rs -ErrorAction SilentlyContinue | Stop-Process -Force" 2>/dev/null; sleep 1; }

# $1 nombre, $2 env, $3 espera antes de medir, $4 duracion medida, $5 tecla opcional, $6 ms de tecla
escenario() {
  local nombre="$1" envs="$2" t0="$3" dur="$4" tecla="$5" ms="$6"
  matar
  mkdir -p "$BASE/$nombre"
  cd "$BASE/$nombre" || exit 1
  rm -f input_log.txt
  env RD_INPUT_LOG=1 $envs "$EXE" > /dev/null 2>&1 &
  if [ -n "$tecla" ]; then
    sleep "$t0"
    powershell -NoProfile -ExecutionPolicy Bypass -File "$PROJ/analisis/teclas.ps1" -Title "RubyDung RS" -Key "$tecla" -Ms "$ms" > /dev/null 2>&1 &
    sleep "$dur"
  else
    sleep $(python -c "print($t0+$dur)")
  fi
  matar
  echo "  $nombre listo"
}

escenario aparecer      ""                                          1.8 2.0
escenario correr_saltar "RD_FIJA=1 RD_POS=128.5,128.5,43.0 RD_ROT=0,0"  2.5 2.5 w 2600
escenario cielo         "RD_FIJA=1 RD_POS=128.5,128.5,43.0 RD_ROT=0,85" 1.6 2.2
escenario suelo         "RD_FIJA=1 RD_POS=128.5,128.5,43.0 RD_ROT=0,-85" 1.6 2.2
escenario reseteo_r     "RD_FIJA=1 RD_POS=128.5,128.5,43.0 RD_ROT=0,0"  2.4 2.2 r 300
escenario vacio         "RD_FIJA=1 RD_POS=400,128.5,60.0 RD_ROT=0,0"     2.4 2.2
echo "todos los escenarios hechos"

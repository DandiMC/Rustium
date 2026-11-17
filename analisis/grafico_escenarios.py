# Barras: FPS de rd-132211 en Rustium vs el original de Java, por situacion.
import json
from PIL import Image, ImageDraw, ImageFont

port_raw = json.load(open("analisis/mcmod/fps132211/escenarios_port.json"))
port = {k: (v[0] if isinstance(v, list) else v) for k, v in port_raw.items()}
orig = json.load(open("analisis/mcmod/fps132211/escenarios_orig.json"))

FILAS = [
    ("aparecer", "1. Al aparecer"),
    ("correr_saltar", "2. Corriendo y saltando"),
    ("cielo", "3. Mirando al cielo"),
    ("suelo", "4. Mirando al suelo"),
    ("reseteo_r", "5. Justo al pulsar R"),
    ("vacio", "6. Cayendo al vacio"),
]

W, H = 1280, 800
L, R = 470, 90
T, B = 200, 120
FONT, FONTB = "C:/Windows/Fonts/segoeui.ttf", "C:/Windows/Fonts/segoeuib.ttf"
VERDE, ROJO = (34, 150, 74), (206, 74, 66)


def f(n, t):
    try:
        return ImageFont.truetype(n, t)
    except Exception:
        return ImageFont.load_default()


img = Image.new("RGB", (W, H), (247, 248, 250))
d = ImageDraw.Draw(img)
ANCHO = W - L - R
TOPE = 800


def bx(v):
    return L + (v / TOPE) * ANCHO


d.rounded_rectangle([L - 30, T - 26, W - 30, H - B + 26], radius=18, fill=(255, 255, 255), outline=(224, 227, 232), width=2)

d.text((36, 34), "rd-132211: Rustium (Rust) vs el original (Java)", font=f(FONTB, 40), fill=(30, 34, 40))
d.text((36, 88), "Windows 11 Home (build 26300) · Ryzen 7 5700U (8C/16T) · Radeon Graphics integrada · 11 GB RAM",
       font=f(FONT, 22), fill=(105, 112, 122))
d.text((36, 120), "Juego a 1024x768 · sin vsync · media de los 2 s siguientes a cada accion",
       font=f(FONT, 22), fill=(105, 112, 122))

# leyenda
d.rounded_rectangle([36, 152, 56, 168], radius=4, fill=VERDE)
d.text((64, 159), "Rustium", font=f(FONTB, 22), fill=VERDE, anchor="lm")
d.rounded_rectangle([190, 152, 210, 168], radius=4, fill=ROJO)
d.text((218, 159), "Original de Java", font=f(FONTB, 22), fill=ROJO, anchor="lm")

# rejilla vertical
for v in range(0, TOPE + 1, 200):
    x = bx(v)
    d.line([x, T - 20, x, H - B + 10], fill=(238, 240, 244), width=1)
    d.text((x, H - B + 20), str(v), font=f(FONT, 20), fill=(150, 156, 165), anchor="ma")
d.text((W - 30, H - B + 4), "fps", font=f(FONT, 20), fill=(150, 156, 165), anchor="ra")

alto_fila = (H - B - T) / len(FILAS)
for i, (clave, etiqueta) in enumerate(FILAS):
    y = T + i * alto_fila
    d.text((36, y + alto_fila / 2), etiqueta, font=f(FONT, 24), fill=(60, 66, 74), anchor="lm")
    pv, ov = port.get(clave), orig.get(clave)
    for j, (val, color) in enumerate(((pv, VERDE), (ov, ROJO))):
        if not val:
            continue
        yy = y + 14 + j * 26
        x1 = bx(val)
        d.rounded_rectangle([L, yy, max(L + 6, x1), yy + 22], radius=6, fill=color)
        d.text((x1 + 12, yy + 11), str(val), font=f(FONTB, 22), fill=color, anchor="lm")
    if pv and ov:
        d.text((W - 60, y + alto_fila / 2), f"x{pv/ov:.1f}", font=f(FONTB, 26), fill=(90, 96, 106), anchor="rm")

d.text((36, H - 74), "El original se dispara al pulsar R porque salta a una zona cuyo mundo aun no ha construido; en cuanto lo tiene, vuelve a ~90.",
       font=f(FONT, 19), fill=(135, 141, 150))
d.text((36, H - 46), "Mismo mundo plano, misma camara y mismo equipo. Rustium compila el mundo una vez y lo mantiene en la GPU.",
       font=f(FONT, 19), fill=(135, 141, 150))

img.save("analisis/capturas/fps_escenarios.png")
print("guardado analisis/capturas/fps_escenarios.png")
for clave, etiqueta in FILAS:
    print(f"  {etiqueta:26s} Rustium {port.get(clave):>4}  original {orig.get(clave):>4}")

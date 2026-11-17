# Grafico de FPS: rd-132211 en Rustium vs el original de Java.
# Lee analisis/mcmod/fps132211/port.csv (t,fps) y orig.csv (n,fps).
from PIL import Image, ImageDraw, ImageFont
import os

W, H = 1280, 720
L, R, T, B = 110, 40, 150, 120
FONT = "C:/Windows/Fonts/segoeui.ttf"
FONTB = "C:/Windows/Fonts/segoeuib.ttf"


def f(nombre, tam):
    try:
        return ImageFont.truetype(nombre, tam)
    except Exception:
        return ImageFont.load_default()


def datos(ruta):
    out = []
    for linea in open(ruta):
        linea = linea.strip()
        if not linea:
            continue
        a, b = linea.split(',')
        out.append((float(a), int(b)))
    return out


port = datos("analisis/mcmod/fps132211/port.csv")
orig = datos("analisis/mcmod/fps132211/orig.csv")
max_t = max(max(t for t, _ in port), float(len(orig)))
max_f = max(max(v for _, v in port), max(v for _, v in orig))
tope = 100 * ((int(max_f * 1.12) + 99) // 100)

fondo = (247, 248, 250)
img = Image.new("RGB", (W, H), fondo)
d = ImageDraw.Draw(img)

# panel
d.rounded_rectangle([L - 24, T - 30, W - R + 20, H - B + 30], radius=18, fill=(255, 255, 255), outline=(224, 227, 232), width=2)

# rejilla + ejes
def px(t): return L + (t / max_t) * (W - L - R)
def py(v): return H - B - (v / tope) * (H - T - B)

for v in range(0, tope, 100):
    y = py(v)
    d.line([L, y, W - R, y], fill=(233, 236, 241), width=1)
    d.text((L - 18, y), str(v), font=f(FONT, 22), fill=(120, 126, 135), anchor="rm")
for t in range(0, int(max_t) + 1, 5):
    x = px(t)
    d.line([x, T - 30, x, H - B], fill=(240, 242, 245), width=1)
    d.text((x, H - B + 14), f"{t} s", font=f(FONT, 22), fill=(120, 126, 135), anchor="ma")

d.line([L, H - B, W - R, H - B], fill=(180, 186, 195), width=2)
d.line([L, T - 30, L, H - B], fill=(180, 186, 195), width=2)

# series
VERDE = (34, 150, 74)
ROJO = (206, 74, 66)

def polilinea(pts, color):
    if len(pts) < 2:
        return
    d.line(pts, fill=color, width=4, joint="curve")

def area(pts, color):
    if len(pts) < 2:
        return
    capa = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    dc = ImageDraw.Draw(capa)
    dc.polygon([(pts[0][0], H - B)] + pts + [(pts[-1][0], H - B)], fill=color + (38,))
    img.paste(Image.alpha_composite(img.convert("RGBA"), capa).convert("RGB"))

pp = [(px(t), py(v)) for t, v in port if t <= max_t and not (t < 2 and v < 30)]
op = [(px(t), py(v)) for t, v in orig]
area(pp, VERDE)
polilinea(op, ROJO)
polilinea(pp, VERDE)

# etiquetas al final de cada serie
def ultimo(pts):
    return pts[-1]

lx, ly = ultimo(pp)
d.ellipse([lx - 7, ly - 7, lx + 7, ly + 7], fill=VERDE)

lx2, ly2 = ultimo(op)
d.ellipse([lx2 - 7, ly2 - 7, lx2 + 7, ly2 + 7], fill=ROJO)


# titulos
d.text((L - 24, 40), "rd-132211: Rustium (Rust) vs el original (Java)", font=f(FONTB, 40), fill=(30, 34, 40))
d.text((L - 24, 98), "FPS por segundo · 1024x768 · mismo mundo y misma camara",
       font=f(FONT, 22), fill=(110, 116, 126))

# cajas de media
mp = sum(v for _, v in port) / len(port)
mo = sum(v for _, v in orig) / len(orig)
def caja(x, y, titulo, valor, color):
    d.rounded_rectangle([x, y, x + 250, y + 104], radius=14, fill=(255, 255, 255), outline=color, width=3)
    d.text((x + 18, y + 14), titulo, font=f(FONT, 24), fill=(110, 116, 126))
    d.text((x + 18, y + 46), valor, font=f(FONTB, 44), fill=color)
caja(W - R - 550, T - 22, "Rustium", f"{mp:.0f} fps", VERDE)
caja(W - R - 280, T - 22, "Original", f"{mo:.0f} fps", ROJO)

d.text((L - 24, H - 74), "Medido en el mismo equipo, sin vsync, con la camara fija en el mismo punto del mundo.",
       font=f(FONT, 20), fill=(130, 136, 145))
d.text((L - 24, H - 46), "El original arranca alto y cae al ir reconstruyendo chunks; Rustium construye el mundo una vez y se queda plano.",
       font=f(FONT, 20), fill=(130, 136, 145))

os.makedirs("analisis/capturas", exist_ok=True)
img.save("analisis/capturas/fps_rd132211.png")
print("guardado analisis/capturas/fps_rd132211.png")
print("Rustium %.0f fps (max %d) | Original %.0f fps (max %d)" % (mp, max(v for _, v in port), mo, max(v for _, v in orig)))

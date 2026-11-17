# rd-132211: arnes de careo contra el original.
#
# IMPORTANTE (leccion aprendida): NO hay que parchear Tesselator ni Chunk. En este
# equipo el jar original arranca y dibuja perfectamente; una copia recompilada
# entera con el Tesselator en modo inmediato no dibujaba nada (solo el color de
# cielo) y me hizo creer que el original no pintaba. Por eso aqui se recompilan
# SOLO dos clases y el resto (Tesselator, Chunk, LevelRenderer, Tile...) sale del
# jar tal cual:
#   RubyDung -> camara fija por -Dcareo.x/y/z/yaw/pitch + traza de posicion.
#   Level    -> volcado de bloques y luces del mundo recien generado.
#
# Uso:
#   python analisis/parche_orig_132211.py           # prepara y compila
#   (lanzar con classes_cam132211 DELANTE del jar, ver cabecera del .cmd)
import glob
import io
import os
import shutil
import subprocess
import sys

ORIG = 'analisis/src/rd-132211'
BUILD = 'analisis/orig_build/src132211'
CLASSES = 'analisis/orig_build/classes_cam132211'


def parche(rel, cambios):
    p = os.path.join(BUILD, rel)
    s = io.open(p, encoding='utf-8').read()
    for viejo, nuevo in cambios:
        if viejo not in s:
            print('NO ENCONTRADO en', rel, ':', viejo.strip().splitlines()[0][:60])
            sys.exit(1)
        s = s.replace(viejo, nuevo, 1)
    io.open(p, 'w', encoding='utf-8', newline='').write(s)


if os.path.isdir(BUILD):
    shutil.rmtree(BUILD)
shutil.copytree(ORIG, BUILD)

# ---- camara fija + traza ----
parche('com/mojang/rubydung/RubyDung.java', [
    ("    private HitResult hitResult = null;",
     """    private HitResult hitResult = null;
    static int FRAMES = 0;"""),
    ("    public void render(float a) {",
     """    public void render(float a) {
        // CAMARA FIJA (careo): la misma que la del port (RD_POS/RD_ROT).
        if (System.getProperty("careo.x") != null) {
            this.player.x = Float.parseFloat(System.getProperty("careo.x"));
            this.player.y = Float.parseFloat(System.getProperty("careo.y"));
            this.player.z = Float.parseFloat(System.getProperty("careo.z"));
            this.player.xo = this.player.x;
            this.player.yo = this.player.y;
            this.player.zo = this.player.z;
            this.player.yRot = Float.parseFloat(System.getProperty("careo.yaw", "0"));
            this.player.xRot = Float.parseFloat(System.getProperty("careo.pitch", "0"));
        }"""),
    ("        Display.update();",
     """        Display.update();
        if (++FRAMES % 30 == 0) {
            System.out.println("POS " + this.player.x + " alto=" + this.player.y + " fondo=" + this.player.z
                + " yaw=" + this.player.yRot + " pitch=" + this.player.xRot);
        }"""),
])

# ---- volcado del mundo ----
p = os.path.join(BUILD, 'com/mojang/rubydung/level/Level.java')
s = io.open(p, encoding='utf-8').read()
viejo = "        this.calcLightDepths(0, 0, w, h);\n        this.load();"
if viejo not in s:
    print('NO ENCONTRADO: constructor de Level')
    sys.exit(1)
s = s.replace(viejo, """        this.calcLightDepths(0, 0, w, h);
        // PARCHE CAREO: volcar el mundo recien generado (antes de load()).
        try {
            java.io.FileOutputStream fo = new java.io.FileOutputStream("bloques_orig.raw");
            fo.write(this.blocks);
            fo.close();
            java.io.DataOutputStream dl = new java.io.DataOutputStream(new java.io.FileOutputStream("luces_orig.raw"));
            for (int li = 0; li < this.lightDepths.length; ++li) {
                dl.writeInt(this.lightDepths[li]);
            }
            dl.close();
        } catch (Exception e) {
            e.printStackTrace();
        }
        this.load();""", 1)
io.open(p, 'w', encoding='utf-8', newline='').write(s)

# ---- compilar solo esas dos clases contra el jar ----
appdata = os.environ['APPDATA']
jar = os.path.join(appdata, '.minecraft', 'versions', 'rd-132211', 'rd-132211.jar')
libs = []
for ruta in ('org/lwjgl/lwjgl/lwjgl', 'org/lwjgl/lwjgl/lwjgl_util', 'net/java/jinput/jinput'):
    encontrados = glob.glob(os.path.join(appdata, '.minecraft', 'libraries', ruta, '*', '*.jar'))
    libs.extend(encontrados)
if os.path.isdir(CLASSES):
    shutil.rmtree(CLASSES)
os.makedirs(CLASSES)
cp = ';'.join([os.path.abspath(jar)] + [os.path.abspath(l) for l in libs])
fuentes = [
    os.path.join(BUILD, 'com/mojang/rubydung/RubyDung.java'),
    os.path.join(BUILD, 'com/mojang/rubydung/level/Level.java'),
]
r = subprocess.run(['javac', '-nowarn', '-cp', cp, '-d', os.path.abspath(CLASSES)] + fuentes)
if r.returncode != 0:
    print('javac fallo')
    sys.exit(1)
print('listo: clases en', CLASSES)
print('classpath: classes_cam132211;%s;<libs lwjgl/jinput>' % jar)

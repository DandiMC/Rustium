# rd-132328: arnes de careo contra el original.
#
# Igual que el de rd-132211: se recompilan SOLO las clases que hacen falta
# (RubyDung, Level y Zombie) y el resto sale del jar original. Asi el Tesselator,
# las display lists y los resources son los del jar.
#
# Uso:
#   python analisis/parche_orig_132328.py
#   java -Dcareo.x=128.5 -Dcareo.y=43.9 -Dcareo.z=128.5 -Dcareo.yaw=0 ...
#        -cp "analisis/orig_build/classes132328;<jar>;<lwjgl>" com.mojang.rubydung.RubyDung
import glob
import io
import os
import shutil
import subprocess
import sys

ORIG = 'analisis/src/rd-132328'
BUILD = 'analisis/orig_build/src132328'
CLASSES = 'analisis/orig_build/classes132328'


def parche(rel, cambios):
    p = os.path.join(BUILD, rel)
    s = io.open(p, encoding='utf-8').read()
    for viejo, nuevo in cambios:
        if viejo not in s:
            print('NO ENCONTRADO en', rel, ':', viejo.strip().splitlines()[0][:70])
            sys.exit(1)
        s = s.replace(viejo, nuevo, 1)
    io.open(p, 'w', encoding='utf-8', newline='').write(s)


if os.path.isdir(BUILD):
    shutil.rmtree(BUILD)
shutil.copytree(ORIG, BUILD)

# ---- RubyDung: camara fija, numero de zombis, zombi fijo, freeze y traza ----
parche('com/mojang/rubydung/RubyDung.java', [
    ("import com.mojang.rubydung.level.LevelRenderer;",
     "import com.mojang.rubydung.level.LevelRenderer;\nimport com.mojang.rubydung.phys.AABB;"),
    ("    private HitResult hitResult = null;",
     """    private HitResult hitResult = null;
    static int FRAMES = 0;"""),
    ("""        int i = 0;
        while (i < 100) {
            this.zombies.add(new Zombie(this.level, 128.0f, 0.0f, 128.0f));
            ++i;
        }""",
     """        int nZ = Integer.getInteger("careo.zombies", 100);
        int i = 0;
        while (i < nZ) {
            this.zombies.add(new Zombie(this.level, 128.0f, 0.0f, 128.0f));
            ++i;
        }
        // Un zombi fijo, con su caja buena, para comparar el modelo.
        if (System.getProperty("careo.zombie.x") != null) {
            float zx = Float.parseFloat(System.getProperty("careo.zombie.x"));
            float zy = Float.parseFloat(System.getProperty("careo.zombie.y"));
            float zz = Float.parseFloat(System.getProperty("careo.zombie.z"));
            Zombie fz = new Zombie(this.level, zx, zy, zz);
            fz.bb = new AABB(zx - 0.3f, zy - 0.9f, zz - 0.3f,
                             zx + 0.3f, zy + 0.9f, zz + 0.3f);
            // Sin esto el render interpola desde (0,0,0) y el zombi sale lejos.
            fz.xo = zx; fz.yo = zy; fz.zo = zz;
            if (System.getProperty("careo.zombie.rot") != null) {
                fz.rot = Float.parseFloat(System.getProperty("careo.zombie.rot"));
            }
            this.zombies.add(fz);
        }"""),
    ("""    public void tick() {
        int i = 0;""",
     """    public void tick() {
        if (System.getProperty("careo.freeze") != null) return;
        int i = 0;"""),
    ("""    public void render(float a) {
        float xo = Mouse.getDX();""",
     """    public void render(float a) {
        // CAMARA FIJA (careo): la misma que la del port.
        if (System.getProperty("careo.x") != null) {
            this.player.x = Float.parseFloat(System.getProperty("careo.x"));
            this.player.y = Float.parseFloat(System.getProperty("careo.y"));
            this.player.z = Float.parseFloat(System.getProperty("careo.z"));
            this.player.xo = this.player.x;
            this.player.yo = this.player.y;
            this.player.zo = this.player.z;
            this.player.yRot = Float.parseFloat(System.getProperty("careo.yaw", "0"));
            // La caja tambien sigue a la camara: pick() solo prueba bb.grow(3).
            this.player.bb = new AABB(this.player.x - 0.3f, this.player.y - 1.62f, this.player.z - 0.3f,
                                      this.player.x + 0.3f, this.player.y + 0.18f, this.player.z + 0.3f);
            this.player.xRot = Float.parseFloat(System.getProperty("careo.pitch", "0"));
        }
        float xo = Mouse.getDX();"""),
    ("        Display.update();",
     """        Display.update();
        if (++FRAMES % 30 == 0) {
            System.out.println("POS " + this.player.x + " alto=" + this.player.y + " fondo=" + this.player.z
                + " yaw=" + this.player.yRot + " pitch=" + this.player.xRot);
            if (!this.zombies.isEmpty()) {
                Zombie z0 = this.zombies.get(0);
                System.out.println("Z " + z0.x + " alto=" + z0.y + " fondo=" + z0.z + " rot=" + z0.rot
                    + " suelo=" + z0.onGround);
            }
        }"""),
])

# ---- Level: volcado del mundo recien generado ----
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
        this.load();
        // PARCHE CAREO: volcar el mundo ya cargado, para comparar level.dat.
        try {
            if (System.getProperty("careo.dump") != null) {
                java.io.FileOutputStream fc = new java.io.FileOutputStream("bloques_cargados_orig.raw");
                fc.write(this.blocks);
                fc.close();
                java.io.DataOutputStream fl = new java.io.DataOutputStream(new java.io.FileOutputStream("luces_cargadas_orig.raw"));
                for (int li = 0; li < this.lightDepths.length; ++li) {
                    fl.writeInt(this.lightDepths[li]);
                }
                fl.close();
            }
        } catch (Exception e) {
            e.printStackTrace();
        }""", 1)
io.open(p, 'w', encoding='utf-8', newline='').write(s)

# ---- Zombie: tiempo de animacion fijo y rumbo fijo para careo ----
parche('com/mojang/rubydung/character/Zombie.java', [
    ("        double time = (double)System.nanoTime() / 1.0E9 * 10.0 * (double)this.speed + (double)this.timeOffs;",
     """        double time = (double)System.nanoTime() / 1.0E9 * 10.0 * (double)this.speed + (double)this.timeOffs;
        if (System.getProperty("careo.time") != null) {
            time = Double.parseDouble(System.getProperty("careo.time"));
        }"""),
    ("""        this.rotA = (float)((double)this.rotA + (Math.random() - Math.random()) * Math.random() * Math.random() * (double)0.01f);
        xa = (float)Math.sin(this.rot);""",
     """        this.rotA = (float)((double)this.rotA + (Math.random() - Math.random()) * Math.random() * Math.random() * (double)0.01f);
        if (System.getProperty("careo.rot") != null) {
            this.rot = Float.parseFloat(System.getProperty("careo.rot"));
            this.rotA = 0.0f;
        }
        xa = (float)Math.sin(this.rot);"""),
])

# ---- compilar solo esas clases contra el jar ----
appdata = os.environ['APPDATA']
jar = os.path.join(appdata, '.minecraft', 'versions', 'rd-132328', 'rd-132328.jar')
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
    os.path.join(BUILD, 'com/mojang/rubydung/character/Zombie.java'),
]
r = subprocess.run(['javac', '-nowarn', '-cp', cp, '-d', os.path.abspath(CLASSES)] + fuentes)
if r.returncode != 0:
    print('javac fallo')
    sys.exit(1)
print('listo: clases en', CLASSES)

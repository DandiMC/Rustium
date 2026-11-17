package rustium.bench;

import java.lang.instrument.ClassFileTransformer;
import java.lang.instrument.Instrumentation;
import java.security.ProtectionDomain;

/**
 * Patches two classes of the game while they are being loaded.
 *
 *  - RubyDung: the operand of "invokestatic Display.update()V" is pointed at
 *    Bench.tick(), which calls the real update and counts the frame; the two
 *    "invokestatic Mouse.getDX/getDY()I" are pointed at Bench, which returns 0,
 *    so the camera stays where the spawn left it instead of tilting with the
 *    pointer the moment the window grabs it.
 *  - Player: "invokestatic Math.random()D" is pointed at Bench.rand(), which
 *    returns a fixed value, so the spawn point is always the same and the frame
 *    rate can be compared against the Rust port from the very same spot.
 *
 * Both are the same instruction with the same size, so no byte of the method
 * moves and the classes keep verifying as they came from Mojang. The jar on disk
 * is never touched.
 */
public final class Agent implements ClassFileTransformer {

    private static final String RUBYDUNG = "com/mojang/rubydung/RubyDung";
    private static final String PLAYER = "com/mojang/rubydung/Player";

    public static void premain(String args, Instrumentation inst) {
        inst.addTransformer(new Agent());
    }

    @Override
    public byte[] transform(ClassLoader loader, String name, Class<?> c,
                            ProtectionDomain pd, byte[] buf) {
        if (buf == null) {
            return null;
        }
        try {
            if (RUBYDUNG.equals(name)) {
                byte[] out = retarget(buf, "org/lwjgl/opengl/Display", "update", "()V", "tick", "()V");
                out = retarget(out, "org/lwjgl/input/Mouse", "getDX", "()I", "getDX", "()I");
                return retarget(out, "org/lwjgl/input/Mouse", "getDY", "()I", "getDY", "()I");
            }
            if (PLAYER.equals(name)) {
                return retarget(buf, "java/lang/Math", "random", "()D", "rand", "()D");
            }
        } catch (Throwable t) {
            System.err.println("Rustium bench: no se pudo instrumentar " + name + ": " + t);
        }
        return null;
    }

    private static int u2(byte[] b, int p) {
        return ((b[p] & 0xFF) << 8) | (b[p + 1] & 0xFF);
    }

    private static int u4(byte[] b, int p) {
        return ((b[p] & 0xFF) << 24) | ((b[p + 1] & 0xFF) << 16)
                | ((b[p + 2] & 0xFF) << 8) | (b[p + 3] & 0xFF);
    }

    /** Repoints every invokestatic of owner.name(desc) to Bench.hook(hookDesc). */
    static byte[] retarget(byte[] b, String owner, String name, String desc,
                           String hook, String hookDesc) throws Exception {
        if (u4(b, 0) != 0xCAFEBABE) {
            throw new IllegalStateException("no es un .class");
        }
        int count = u2(b, 8);
        int[] tag = new int[count];
        String[] utf = new String[count];
        int[] first = new int[count];
        int[] second = new int[count];

        int p = 10;
        for (int i = 1; i < count; i++) {
            int t = b[p] & 0xFF;
            tag[i] = t;
            p++;
            switch (t) {
                case 1: {
                    int len = u2(b, p);
                    utf[i] = new String(b, p + 2, len, "UTF-8");
                    p += 2 + len;
                    break;
                }
                case 7: case 8: case 16:
                    first[i] = u2(b, p);
                    p += 2;
                    break;
                case 15:
                    p += 3;
                    break;
                case 3: case 4:
                    p += 4;
                    break;
                case 9: case 10: case 11: case 12: case 17: case 18:
                    first[i] = u2(b, p);
                    second[i] = u2(b, p + 2);
                    p += 4;
                    break;
                case 5: case 6:
                    p += 8;
                    i++;
                    break;
                default:
                    throw new IllegalStateException("tag de constant pool " + t);
            }
        }
        int poolEnd = p;

        int target = -1;
        for (int i = 1; i < count; i++) {
            if (tag[i] != 10) {
                continue;
            }
            int cls = first[i];
            int nat = second[i];
            if (cls <= 0 || cls >= count || tag[cls] != 7) {
                continue;
            }
            if (!owner.equals(utf[first[cls]])) {
                continue;
            }
            if (nat <= 0 || nat >= count || tag[nat] != 12) {
                continue;
            }
            if (name.equals(utf[first[nat]]) && desc.equals(utf[second[nat]])) {
                target = i;
                break;
            }
        }
        if (target < 0) {
            throw new IllegalStateException("no hay llamada a " + owner + "." + name + desc);
        }

        // Append the constants for our own call.
        int n = count;
        int utfBench = n++;
        int clsBench = n++;
        int utfHook = n++;
        int utfDesc = n++;
        int natHook = n++;
        int refHook = n++;

        java.io.ByteArrayOutputStream extra = new java.io.ByteArrayOutputStream();
        writeUtf(extra, "rustium/bench/Bench");
        extra.write(7); writeU2(extra, utfBench);
        writeUtf(extra, hook);
        writeUtf(extra, hookDesc);
        extra.write(12); writeU2(extra, utfHook); writeU2(extra, utfDesc);
        extra.write(10); writeU2(extra, clsBench); writeU2(extra, natHook);
        byte[] extraBytes = extra.toByteArray();

        java.io.ByteArrayOutputStream out = new java.io.ByteArrayOutputStream(b.length + extraBytes.length);
        out.write(b, 0, 8);
        writeU2(out, n);
        out.write(b, 10, poolEnd - 10);
        out.write(extraBytes);
        out.write(b, poolEnd, b.length - poolEnd);

        byte[] r = out.toByteArray();
        int base = poolEnd + extraBytes.length;
        fixCode(r, base, target, refHook);
        return r;
    }

    /** Walks the class body and repoints every invokestatic at the target. */
    private static void fixCode(byte[] r, int p, int target, int replacement) {
        p += 2;                 // access_flags
        p += 2 + 2;             // this_class, super_class
        int ifaces = u2(r, p);
        p += 2 + ifaces * 2;
        p = skipMembers(r, p);  // fields
        int methods = u2(r, p);
        p += 2;
        for (int m = 0; m < methods; m++) {
            p += 6;             // access, name, desc
            int attrs = u2(r, p);
            p += 2;
            for (int a = 0; a < attrs; a++) {
                String name = utfAt(r, u2(r, p));
                int len = u4(r, p + 2);
                int body = p + 6;
                if ("Code".equals(name)) {
                    int codeLen = u4(r, body + 4);
                    int code = body + 8;
                    for (int i = 0; i + 2 < codeLen; i++) {
                        if ((r[code + i] & 0xFF) == 0xB8 && u2(r, code + i + 1) == target) {
                            r[code + i + 1] = (byte) (replacement >> 8);
                            r[code + i + 2] = (byte) replacement;
                        }
                    }
                }
                p = body + len;
            }
        }
    }

    private static int skipMembers(byte[] r, int p) {
        int count = u2(r, p);
        p += 2;
        for (int i = 0; i < count; i++) {
            p += 6;
            int attrs = u2(r, p);
            p += 2;
            for (int a = 0; a < attrs; a++) {
                int len = u4(r, p + 2);
                p += 6 + len;
            }
        }
        return p;
    }

    /** Reads a Utf8 entry out of the constant pool of a .class image. */
    private static String utfAt(byte[] b, int index) {
        int count = u2(b, 8);
        int p = 10;
        for (int i = 1; i < count; i++) {
            int t = b[p] & 0xFF;
            p++;
            if (t == 1) {
                int len = u2(b, p);
                if (i == index) {
                    return new String(b, p + 2, len, java.nio.charset.StandardCharsets.UTF_8);
                }
                p += 2 + len;
            } else if (t == 7 || t == 8 || t == 16) {
                p += 2;
            } else if (t == 15) {
                p += 3;
            } else if (t == 3 || t == 4) {
                p += 4;
            } else if (t == 9 || t == 10 || t == 11 || t == 12 || t == 17 || t == 18) {
                p += 4;
            } else if (t == 5 || t == 6) {
                p += 8;
                i++;
            }
        }
        return null;
    }

    private static void writeUtf(java.io.ByteArrayOutputStream o, String s) throws Exception {
        byte[] b = s.getBytes("UTF-8");
        o.write(1);
        writeU2(o, b.length);
        o.write(b);
    }

    private static void writeU2(java.io.ByteArrayOutputStream o, int v) {
        o.write((v >> 8) & 0xFF);
        o.write(v & 0xFF);
    }
}

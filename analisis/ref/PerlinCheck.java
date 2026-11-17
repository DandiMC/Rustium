// Copia del PerlinNoiseFilter de rd-20090515 (descompilado) con semilla fija,
// para poder comparar numeros exactos con el port de Rust.
// Uso: javac PerlinCheck.java && java PerlinCheck 12345 0
public class PerlinCheck {
    static class Rand extends java.util.Random {
        Rand(long seed) { super(seed); }
    }

    int levels = 0;
    int fuzz = 16;

    PerlinCheck(int levels) { this.levels = levels; }

    public int[] read(int width, int height, long seed) {
        int x;
        java.util.Random random = new Rand(seed);
        int[] tmp = new int[width * height];
        int level = this.levels;
        int step = width >> level;
        int y = 0;
        while (y < height) {
            x = 0;
            while (x < width) {
                tmp[x + y * width] = (random.nextInt(256) - 128) * this.fuzz;
                x += step;
            }
            y += step;
        }
        step = width >> level;
        while (step > 1) {
            int x2;
            int val = 256 * (step << level);
            int ss = step / 2;
            int y2 = 0;
            while (y2 < height) {
                x2 = 0;
                while (x2 < width) {
                    int ul = tmp[(x2 + 0) % width + (y2 + 0) % height * width];
                    int ur = tmp[(x2 + step) % width + (y2 + 0) % height * width];
                    int dl = tmp[(x2 + 0) % width + (y2 + step) % height * width];
                    int dr = tmp[(x2 + step) % width + (y2 + step) % height * width];
                    tmp[x2 + ss + (y2 + ss) * width] = (ul + dl + ur + dr) / 4 + random.nextInt(val * 2) - val;
                    x2 += step;
                }
                y2 += step;
            }
            y2 = 0;
            while (y2 < height) {
                x2 = 0;
                while (x2 < width) {
                    int c = tmp[x2 + y2 * width];
                    int r = tmp[(x2 + step) % width + y2 * width];
                    int d = tmp[x2 + (y2 + step) % width * width];
                    int mu = tmp[(x2 + ss & width - 1) + (y2 + ss - step & height - 1) * width];
                    int ml = tmp[(x2 + ss - step & width - 1) + (y2 + ss & height - 1) * width];
                    int m = tmp[(x2 + ss) % width + (y2 + ss) % height * width];
                    int u = (c + r + m + mu) / 4 + random.nextInt(val * 2) - val;
                    int l = (c + d + m + ml) / 4 + random.nextInt(val * 2) - val;
                    tmp[x2 + ss + y2 * width] = u;
                    tmp[x2 + (y2 + ss) * width] = l;
                    x2 += step;
                }
                y2 += step;
            }
            step /= 2;
        }
        int[] result = new int[width * height];
        y = 0;
        while (y < height) {
            x = 0;
            while (x < width) {
                result[x + y * width] = tmp[x % width + y % height * width] / 512 + 128;
                ++x;
            }
            ++y;
        }
        return result;
    }

    public static void main(String[] args) {
        long seed = Long.parseLong(args[0]);
        int levels = Integer.parseInt(args[1]);
        int w = 256, h = 256;
        int[] r = new PerlinCheck(levels).read(w, h, seed);
        long sum = 0;
        for (int v : r) sum += v;
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < 8; i++) sb.append(r[i]).append(',');
        System.out.println("levels=" + levels + " suma=" + sum + " primeros8=" + sb);
    }
}

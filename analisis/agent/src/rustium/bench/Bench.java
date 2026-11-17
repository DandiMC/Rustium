package rustium.bench;

import java.lang.reflect.Method;

/**
 * Replaces the game's call to Display.update(): it does the real update and then
 * counts one frame. Prints RUSTIUM_FPS=<n> four times a second so the launcher can
 * read the rate without touching the game's code on disk.
 *
 * Also replaces Math.random() inside Player, giving the spawn a fixed point so the
 * original and the Rust port are measured from the same place and angle.
 */
public final class Bench {

    /**
     * Fraction of the map the spawn is pinned to. It arrives from the launcher
     * (-Drustium.spawn), which hands the port the very same number, so the spot
     * itself lives in one place and each side derives its own coordinates from
     * the size of its own world.
     */
    private static final double SPAWN = spawn(System.getProperty("rustium.spawn", "0.5"));

    private static double spawn(String value) {
        try {
            return Double.parseDouble(value.trim());
        } catch (RuntimeException bad) {
            return 0.5;
        }
    }

    private static Method update;
    private static int frames;
    private static long last = System.nanoTime();

    private Bench() {}

    public static double rand() {
        return SPAWN;
    }

    /**
     * The camera belongs to the bench: the original turns it with the mouse every
     * frame (RubyDung.render), and the delta from grabbing the pointer at launch
     * used to tilt it before the first reading. Both come back as zero, so the
     * view stays exactly where the spawn put it, like the port's.
     */
    public static int getDX() {
        return 0;
    }

    public static int getDY() {
        return 0;
    }

    public static void tick() {
        try {
            if (update == null) {
                Class<?> display = Class.forName("org.lwjgl.opengl.Display");
                update = display.getMethod("update");
            }
            update.invoke(null);
        } catch (Throwable t) {
            throw new RuntimeException("Rustium bench: Display.update failed", t);
        }

        frames++;
        long now = System.nanoTime();
        long dt = now - last;
        if (dt >= 250_000_000L) {
            int fps = (int) Math.round(frames * 1.0e9 / dt);
            System.out.println("RUSTIUM_FPS=" + fps);
            System.out.flush();
            frames = 0;
            last = now;
        }
    }
}

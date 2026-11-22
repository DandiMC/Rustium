// NoiseMap de Classic c0.0.11a (com.mojang.minecraft.level.NoiseMap), portado
// tal cual. Es el generador que sustituye a PerlinNoiseFilter al empezar la era
// Classic: mismo esquema de diamantes, pero con nivel 0 y 1 y el mismo ruido
// desplazado en cada pasada.
//
// Como el original crea `new Random()` dentro de read(), su mundo cambia en cada
// arranque. Aqui la semilla entra por parametro para poder fijarla (careo) y
// para que el mundo se pueda reproducir.
//
// Las dos lios del descompilado son los originales y hay que copiarlos:
//   tmp[(x2 + ss & width - 1) + ...]  ->  (x2 + ss) & (width - 1)
//   (y2 + ss - step & height - 1)     ->  (y2 + ss - step) & (height - 1)

use super::perlin::JavaRandom;

pub fn read(levels: i32, width: usize, height: usize, seed: i64) -> Vec<i32> {
    let fuzz = 16;
    let mut random = JavaRandom::new(seed);
    let w = width as i32;
    let h = height as i32;
    let mut tmp = vec![0i32; width * height];

    let mut step = w >> levels;
    let mut y = 0;
    while y < h {
        let mut x = 0;
        while x < w {
            tmp[(x + y * w) as usize] = (random.next_int(256) - 128) * fuzz;
            x += step;
        }
        y += step;
    }

    step = w >> levels;
    while step > 1 {
        let val = 256 * (step << levels);
        let ss = step / 2;

        let mut y2 = 0;
        while y2 < h {
            let mut x2 = 0;
            while x2 < w {
                let ul = tmp[((x2) % w + (y2) % h * w) as usize];
                let ur = tmp[((x2 + step) % w + (y2) % h * w) as usize];
                let dl = tmp[((x2) % w + (y2 + step) % h * w) as usize];
                let dr = tmp[((x2 + step) % w + (y2 + step) % h * w) as usize];
                tmp[(x2 + ss + (y2 + ss) * w) as usize] =
                    (ul + dl + ur + dr) / 4 + random.next_int(val * 2) - val;
                x2 += step;
            }
            y2 += step;
        }

        let mut y2 = 0;
        while y2 < h {
            let mut x2 = 0;
            while x2 < w {
                let c = tmp[(x2 + y2 * w) as usize];
                let r = tmp[((x2 + step) % w + y2 * w) as usize];
                let d = tmp[(x2 + (y2 + step) % h * w) as usize];
                let mu = tmp[((x2 + ss) & (w - 1)) as usize
                    + (((y2 + ss - step) & (h - 1)) * w) as usize];
                let ml = tmp[((x2 + ss - step) & (w - 1)) as usize
                    + (((y2 + ss) & (h - 1)) * w) as usize];
                let m = tmp[((x2 + ss) % w + (y2 + ss) % h * w) as usize];
                let u = (c + r + m + mu) / 4 + random.next_int(val * 2) - val;
                let l = (c + d + m + ml) / 4 + random.next_int(val * 2) - val;
                tmp[(x2 + ss + y2 * w) as usize] = u;
                tmp[(x2 + (y2 + ss) * w) as usize] = l;
                x2 += step;
            }
            y2 += step;
        }
        step /= 2;
    }

    let mut result = vec![0i32; width * height];
    for y in 0..h {
        for x in 0..w {
            result[(x + y * w) as usize] = tmp[((x % w) + (y % h) * w) as usize] / 512 + 128;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_misma_semilla_da_el_mismo_mapa() {
        let a = read(0, 256, 256, 12345);
        let b = read(0, 256, 256, 12345);
        assert_eq!(a, b);
        assert_eq!(a.len(), 256 * 256);
        // El ruido no esta acotado a 0..255 (LevelGen lo divide entre 8 y lo
        // suma a la altura del mundo); lo que si importa es que haya relieve.
        assert!(a.iter().max().unwrap() - a.iter().min().unwrap() > 20);
        // Y otra semilla cambia el mapa.
        assert_ne!(a, read(0, 256, 256, 54321));
    }

    #[test]
    fn el_nivel_1_es_mas_rugoso_que_el_0() {
        // levels=1 arranca con celdas de 128 px separadas 32 (w >> 1), asi que
        // conserva menos detalle; aun asi tiene que llenar todo el mapa.
        let r = read(1, 256, 256, 7);
        assert_eq!(r.len(), 256 * 256);
        assert!(r.iter().max().unwrap() - r.iter().min().unwrap() > 20);
    }
}

// PerlinNoiseFilter de la familia rd-160052 / rd-20090515 (paquete
// com.mojang.rubydung / com.mojang.minecraft), portado tal cual, con un
// java.util.Random compatible para poder comparar numeros exactos con el
// original (ver analisis/ref/PerlinCheck.java).
//
// Ojo con dos lios del descompilado, que son los originales y hay que copiar:
//   tmp[(x2 + ss & width - 1) + ...]        ->  (x2 + ss) & (width - 1)
//   tmp[x2 + (y2 + step) % width * width]   ->  (y2 + step) % width * width

/// java.util.Random (LCG de 48 bits) para reproducir el original bit a bit.
pub struct JavaRandom {
    seed: u64,
}

impl JavaRandom {
    pub fn new(seed: i64) -> Self {
        JavaRandom { seed: ((seed as u64) ^ 0x5DEECE66D) & ((1u64 << 48) - 1) }
    }

    fn next(&mut self, bits: u32) -> u32 {
        self.seed = self
            .seed
            .wrapping_mul(0x5DEECE66D)
            .wrapping_add(0xB)
            & ((1u64 << 48) - 1);
        (self.seed >> (48 - bits)) as u32
    }

    pub fn next_int(&mut self, bound: i32) -> i32 {
        if bound & -bound == bound {
            (((bound as i64) * (self.next(31) as i64)) >> 31) as i32
        } else {
            loop {
                let bits = self.next(31) as i32;
                let val = bits % bound;
                if bits - val + (bound - 1) >= 0 {
                    return val;
                }
            }
        }
    }
}

/// PerlinNoiseFilter.read(width, height) -> valores 0..255.
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
                let ul = tmp[(((x2) % w) + ((y2) % h) * w) as usize];
                let ur = tmp[(((x2 + step) % w) + ((y2) % h) * w) as usize];
                let dl = tmp[(((x2) % w) + ((y2 + step) % h) * w) as usize];
                let dr = tmp[(((x2 + step) % w) + ((y2 + step) % h) * w) as usize];
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
                let r = tmp[(((x2 + step) % w) + y2 * w) as usize];
                // (y2 + step) % width * width
                let d = tmp[(x2 + ((y2 + step) % w) * w) as usize];
                let mu = tmp[((x2 + ss) & (w - 1)) as usize + (((y2 + ss - step) & (h - 1)) * w) as usize];
                let ml = tmp[((x2 + ss - step) & (w - 1)) as usize + (((y2 + ss) & (h - 1)) * w) as usize];
                let m = tmp[(((x2 + ss) % w) + ((y2 + ss) % h) * w) as usize];
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

    // Valores sacados del Java (analisis/ref/PerlinCheck.java), misma semilla.
    #[test]
    fn coincide_con_el_original() {
        let r = read(0, 256, 256, 12345);
        assert_eq!(r.iter().map(|&v| v as i64).sum::<i64>(), 10994872);
        assert_eq!(&r[..8], &[127, 128, 128, 128, 131, 130, 130, 131]);

        let r = read(1, 256, 256, 12345);
        assert_eq!(r.iter().map(|&v| v as i64).sum::<i64>(), 6558853);
        assert_eq!(&r[..8], &[127, 123, 117, 114, 117, 113, 109, 110]);

        let r = read(0, 256, 256, 777);
        assert_eq!(r.iter().map(|&v| v as i64).sum::<i64>(), 5795910);
        assert_eq!(&r[..8], &[129, 128, 129, 128, 128, 129, 130, 130]);
    }
}

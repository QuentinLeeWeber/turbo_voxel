use super::{CHUNK_WIDTH, Chunk, Material, perlin_noise::PerlinNoise};
use crate::engine::perlin_noise::PerlinNoiseParams;
use std::alloc;

const PERLIN_SEED: u32 = 123456789;

pub fn generate_chunk(x: i32, y: i32, z: i32) -> Chunk {
    println!("generate chunks");

    let mut chunk = Chunk {
        pos: [x, y, z],
        materials: alloc_materials(),
        amount: alloc_amount(),
    };

    let sea_level = 8.0;
    let mut terrain = ridged_noise(RigedNoiseParams {
        chunk_x: x,
        chunk_y: y,
        amplitude: 40.,
        gradient_spacing: 100,
        seed: PERLIN_SEED,
        octaves: 2,
        lacunarity: 0.5,
        amplitude_gain: 1.,
    });

    for xi in 0..CHUNK_WIDTH {
        for yi in 0..CHUNK_WIDTH {
            terrain[xi][yi] += sea_level;

            for zi in 0..CHUNK_WIDTH {
                let diff = zi as f32 - terrain[xi][yi];
                if diff > 1.0 {
                    chunk.materials[xi][yi][zi] = Material::default();
                    chunk.amount[xi][yi][zi] = 0.0;
                } else if diff > 0.0 {
                    chunk.materials[xi][yi][zi] = Material::default();
                    chunk.amount[xi][yi][zi] = diff;
                } else {
                    chunk.materials[xi][yi][zi] = Material::default();
                    chunk.amount[xi][yi][zi] = 1.0;
                }
            }
        }
    }

    chunk
}

pub struct RigedNoiseParams {
    pub chunk_x: i32,
    pub chunk_y: i32,
    pub amplitude: f32,
    pub gradient_spacing: u32,
    pub seed: u32,
    pub octaves: u32,
    pub lacunarity: f32,
    pub amplitude_gain: f32,
}

fn gen_2d_range(
    from1: usize,
    to1: usize,
    from2: usize,
    to2: usize,
) -> impl Iterator<Item = (usize, usize)> {
    (from1..to1).flat_map(move |a| (from2..to2).map(move |b| (a, b)))
}

fn ridged_noise(params: RigedNoiseParams) -> Box<[[f32; CHUNK_WIDTH]; CHUNK_WIDTH]> {
    let mut map = unsafe {
        let layout = alloc::Layout::new::<[[f32; CHUNK_WIDTH]; CHUNK_WIDTH]>();
        let ptr = alloc::alloc_zeroed(layout) as *mut [[f32; CHUNK_WIDTH]; CHUNK_WIDTH];
        Box::from_raw(ptr)
    };

    let perlin_noise = PerlinNoise::new(PerlinNoiseParams {
        seed: params.seed,
        chunk_x: params.chunk_x,
        chunk_y: params.chunk_y,
        gradient_spacing: params.gradient_spacing,
        chunk_width: CHUNK_WIDTH as u32,
    });

    for x in 0..CHUNK_WIDTH {
        for y in 0..CHUNK_WIDTH {
            let mut amplitude = params.amplitude;
            let mut frequency = 1.0;
            let mut noise_sum = 0.0;
            let mut weight = 1.0;

            for _ in 0..params.octaves {
                let mut signal = perlin_noise
                    .noise((x as f32 * frequency) as i32, (y as f32 * frequency) as i32);
                signal = 1.0 - signal.abs();

                signal *= weight;
                weight = signal.clamp(0.0, 1.0);

                noise_sum += signal * amplitude;
                amplitude *= params.amplitude_gain;
                frequency *= params.lacunarity;
            }

            map[x][y] = noise_sum;
        }
    }

    map
}

fn alloc_amount() -> Box<[[[f32; CHUNK_WIDTH]; CHUNK_WIDTH]; CHUNK_WIDTH]> {
    unsafe {
        let layout = alloc::Layout::new::<[[[f32; CHUNK_WIDTH]; CHUNK_WIDTH]; CHUNK_WIDTH]>();
        let ptr =
            alloc::alloc_zeroed(layout) as *mut [[[f32; CHUNK_WIDTH]; CHUNK_WIDTH]; CHUNK_WIDTH];
        Box::from_raw(ptr)
    }
}

fn alloc_materials() -> Box<[[[Material; CHUNK_WIDTH]; CHUNK_WIDTH]; CHUNK_WIDTH]> {
    unsafe {
        let layout = alloc::Layout::new::<[[[Material; CHUNK_WIDTH]; CHUNK_WIDTH]; CHUNK_WIDTH]>();
        let ptr = alloc::alloc_zeroed(layout)
            as *mut [[[Material; CHUNK_WIDTH]; CHUNK_WIDTH]; CHUNK_WIDTH];
        Box::from_raw(ptr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ridged_noise_continuity() {
        let eps = 1.0;

        let new_chunk = |cx, cy| {
            ridged_noise(RigedNoiseParams {
                chunk_x: cx,
                chunk_y: cy,
                amplitude: 40.,
                gradient_spacing: 100,
                seed: 123,
                octaves: 1,
                lacunarity: 0.5,
                amplitude_gain: 1.,
            })
        };

        let chunk00 = new_chunk(0, 0);
        let chunk10 = new_chunk(1, 0);
        let chunk01 = new_chunk(0, 1);

        for y in 0..CHUNK_WIDTH {
            assert!((chunk00[CHUNK_WIDTH - 1][y] - chunk10[0][y]).abs() < eps);
        }

        for x in 0..CHUNK_WIDTH {
            assert!((chunk00[x][CHUNK_WIDTH - 1] - chunk01[x][0]).abs() < eps);
        }
    }
}

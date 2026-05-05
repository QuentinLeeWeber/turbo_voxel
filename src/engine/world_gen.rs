use super::{Chunk, Material, perlin_noise::PerlinNoise};
use crate::engine::perlin_noise::PerlinNoiseParams;

const PERLIN_SEED: u32 = 123456789;

pub fn generate_chunk(x: i32, y: i32, z: i32) -> Chunk {
    let mut chunk = Chunk {
        pos: [x, y, z],
        materials: Chunk::alloc_materials(),
        amount: Chunk::alloc_amount(),
    };

    let sea_level = 8.0;
    let mut terrain = ridged_noise(RigedNoiseParams {
        chunk_x: x,
        chunk_y: y,
        amplitude: 20.,
        gradient_spacing: 100,
        seed: PERLIN_SEED,
        octaves: 5,
        frequency_gain: 0.5,
        amplitude_gain: 0.5,
        chunk_width: Chunk::WIDTH as u32,
    });

    for xi in 0..Chunk::WIDTH {
        for yi in 0..Chunk::WIDTH {
            terrain[xi][yi] += sea_level;

            for zi in 0..Chunk::WIDTH {
                let world_z = zi as i32 + z * Chunk::WIDTH as i32;

                let amount = (terrain[xi][yi] - world_z as f32).clamp(-1.0, 1.0);

                chunk.materials[xi][yi][zi] = Material::default();
                chunk.amount[xi][yi][zi] = amount;
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
    pub frequency_gain: f32,
    pub amplitude_gain: f32,
    pub chunk_width: u32,
}

fn gen_2d_range(
    from1: usize,
    to1: usize,
    from2: usize,
    to2: usize,
) -> impl Iterator<Item = (usize, usize)> {
    (from1..to1).flat_map(move |a| (from2..to2).map(move |b| (a, b)))
}

fn ridged_noise(params: RigedNoiseParams) -> Vec<Vec<f32>> {
    let mut map = vec![vec![0.0; params.chunk_width as usize]; params.chunk_width as usize];

    let mut amplitude = params.amplitude;
    let mut frequency = 1.0;

    for _ in 0..params.octaves {
        let perlin_noise = PerlinNoise::new(PerlinNoiseParams {
            seed: params.seed,
            chunk_x: params.chunk_x,
            chunk_y: params.chunk_y,
            gradient_spacing: (params.gradient_spacing as f32 * frequency).round() as u32,
            chunk_width: params.chunk_width,
        });

        for x in 0..params.chunk_width {
            for y in 0..params.chunk_width {
                let signal = perlin_noise.noise(x as i32, y as i32);
                map[x as usize][y as usize] += signal * amplitude;
            }
        }

        amplitude *= params.amplitude_gain;
        frequency *= params.frequency_gain;
    }

    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ridged_noise_continuity() {
        let eps = 1.;
        let chunk_width: usize = 64;

        let new_chunk = |cx, cy| {
            ridged_noise(RigedNoiseParams {
                chunk_x: cx,
                chunk_y: cy,
                amplitude: 1.,
                gradient_spacing: 16,
                chunk_width: chunk_width as u32,
                seed: 42,
                octaves: 1,
                frequency_gain: 0.5,
                amplitude_gain: 1.,
            })
        };

        let chunk00 = new_chunk(0, 0);
        let chunk10 = new_chunk(1, 0);
        let chunk01 = new_chunk(0, 1);

        for y in 0..chunk_width {
            assert!((chunk00[chunk_width - 1][y] - chunk10[0][y]).abs() < eps);
        }

        for x in 0..chunk_width {
            assert!((chunk00[x][chunk_width - 1] - chunk01[x][0]).abs() < eps);
        }
    }

    #[test]
    fn test_ridged_noise_continuity_multiple_octaves() {
        let eps = 1.;
        let chunk_width: usize = 64;

        let new_chunk = |cx, cy| {
            ridged_noise(RigedNoiseParams {
                chunk_x: cx,
                chunk_y: cy,
                amplitude: 1.,
                gradient_spacing: 16,
                chunk_width: chunk_width as u32,
                seed: 42,
                octaves: 3,
                frequency_gain: 0.5,
                amplitude_gain: 1.,
            })
        };

        let chunk00 = new_chunk(0, 0);
        let chunk10 = new_chunk(1, 0);
        let chunk01 = new_chunk(0, 1);

        for y in 0..chunk_width {
            assert!((chunk00[chunk_width - 1][y] - chunk10[0][y]).abs() < eps);
        }

        for x in 0..chunk_width {
            assert!((chunk00[x][chunk_width - 1] - chunk01[x][0]).abs() < eps);
        }
    }

    #[test]
    fn test_ridged_noise_vs_perlin_noise_1() {
        let eps = 1e-5;

        let ridged_chunk = ridged_noise(RigedNoiseParams {
            chunk_x: 0,
            chunk_y: 0,
            amplitude: 1.,
            gradient_spacing: 16,
            seed: 42,
            octaves: 1,
            frequency_gain: 0.5,
            amplitude_gain: 1.,
            chunk_width: 64,
        });

        let perlin_chunk = PerlinNoise::new(PerlinNoiseParams {
            seed: 42,
            chunk_x: 0,
            chunk_y: 0,
            gradient_spacing: 16,
            chunk_width: 64,
        });

        for y in 0..64 {
            for x in 0..64 {
                assert!((ridged_chunk[x][y] - perlin_chunk.noise(x as i32, y as i32)).abs() < eps);
            }
        }
    }

    #[test]
    fn test_ridged_noise_vs_perlin_noise_2() {
        let eps = 1e-5;

        let ridged_chunk = ridged_noise(RigedNoiseParams {
            chunk_x: 42,
            chunk_y: 42,
            amplitude: 1.,
            gradient_spacing: 16,
            seed: 123,
            octaves: 1,
            frequency_gain: 0.5,
            amplitude_gain: 1.,
            chunk_width: 64,
        });

        let perlin_chunk = PerlinNoise::new(PerlinNoiseParams {
            seed: 123,
            chunk_x: 42,
            chunk_y: 42,
            gradient_spacing: 16,
            chunk_width: 64,
        });

        for y in 0..64 {
            for x in 0..64 {
                assert!((ridged_chunk[x][y] - perlin_chunk.noise(x as i32, y as i32)).abs() < eps);
            }
        }
    }

    #[test]
    fn test_chunk_difference() {
        let chunk0 = generate_chunk(0, 0, 0);

        for x in 1..3 {
            for y in 1..3 {
                for z in 1..3 {
                    let chunk = generate_chunk(x, y, z);
                    assert!(!(chunk0.amount == chunk.amount));
                }
            }
        }
    }
}

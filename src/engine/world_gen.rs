use super::{Chunk, Material, perlin_noise::PerlinNoise};
use crate::engine::perlin_noise::PerlinNoiseParams;

const PERLIN_SEED: u32 = 123456789;

pub fn generate_chunk(x: i32, y: i32, z: i32) -> Chunk {
    let mut chunk = Chunk {
        pos: [x, y, z],
        materials: Chunk::alloc_materials(),
        amount: Chunk::alloc_amount(),
    };

    let sea_level = 20.0;
    let mut terrain = ridged_noise(RigedNoiseParams {
        chunk_x: x,
        chunk_y: z,
        amplitude: 20.,
        gradient_spacing: 64,
        seed: PERLIN_SEED,
        octaves: 5,
        frequency_gain: 0.5,
        amplitude_gain: 0.5,
        chunk_width: Chunk::WIDTH as u32,
    });

    for xi in 0..Chunk::WIDTH {
        for zi in 0..Chunk::WIDTH {
            terrain[xi][zi] += sea_level;

            for yi in 0..Chunk::WIDTH {
                let world_y = yi as i32 + y * Chunk::WIDTH as i32;

                let amount = (terrain[xi][zi] - world_y as f32).clamp(-1.0, 1.0);

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
    // this is dirty, but since I am the only one working with world generation (hopefully), this is fine
    assert!(params.gradient_spacing > 0 && params.chunk_width % params.gradient_spacing == 0);

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
    #[should_panic]
    fn test_chunk_with_not_multiple_of_gradient_spacing() {
        let _ = ridged_noise(RigedNoiseParams {
            chunk_x: 0,
            chunk_y: 0,
            amplitude: 20.0,
            gradient_spacing: 100,
            seed: PERLIN_SEED,
            octaves: 5,
            frequency_gain: 0.5,
            amplitude_gain: 0.5,
            chunk_width: 128,
        });
    }

    // Run with: cargo test debug_ridged_noise_image -- --nocapture
    #[test]
    fn debug_ridged_noise_image() {
        use image::{ImageBuffer, Rgb};
        use std::fs;

        const CHUNKS_X: usize = 4;
        const CHUNKS_Y: usize = 4;

        const CHUNK_WIDTH: usize = 128;
        const BORDER_PX: usize = 2;

        let img_w = CHUNKS_X * CHUNK_WIDTH + (CHUNKS_X - 1) * BORDER_PX;
        let img_h = CHUNKS_Y * CHUNK_WIDTH + (CHUNKS_Y - 1) * BORDER_PX;

        let mut chunks: Vec<Vec<Vec<f32>>> = Vec::with_capacity(CHUNKS_X * CHUNKS_Y);
        let mut min_val = f32::MAX;
        let mut max_val = f32::MIN;

        for cy in 0..CHUNKS_Y {
            for cx in 0..CHUNKS_X {
                let vals = ridged_noise(RigedNoiseParams {
                    chunk_x: cx as i32,
                    chunk_y: cy as i32,
                    amplitude: 20.0,
                    gradient_spacing: 64,
                    seed: PERLIN_SEED,
                    octaves: 5,
                    frequency_gain: 0.5,
                    amplitude_gain: 0.5,
                    chunk_width: CHUNK_WIDTH as u32,
                });

                for x in 0..CHUNK_WIDTH {
                    for y in 0..CHUNK_WIDTH {
                        let v = vals[x][y];
                        if v < min_val {
                            min_val = v;
                        }
                        if v > max_val {
                            max_val = v;
                        }
                    }
                }
                chunks.push(vals);
            }
        }

        let range = (max_val - min_val).max(1e-6);
        let border_color = Rgb([255u8, 0u8, 0u8]);
        let mut img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_pixel(img_w as u32, img_h as u32, border_color);

        for cy in 0..CHUNKS_Y {
            for cx in 0..CHUNKS_X {
                let ox = cx * (CHUNK_WIDTH + BORDER_PX);
                let oy = cy * (CHUNK_WIDTH + BORDER_PX);
                let vals = &chunks[cy * CHUNKS_X + cx];

                for x in 0..CHUNK_WIDTH {
                    for y in 0..CHUNK_WIDTH {
                        let v = vals[x][y];
                        let byte = ((v - min_val) / range * 255.0) as u8;
                        img.put_pixel((ox + x) as u32, (oy + y) as u32, Rgb([byte, byte, byte]));
                    }
                }
            }
        }

        fs::create_dir_all("test_output").expect("Could not create test_output/");
        img.save("test_output/debug_ridged_noise.png")
            .expect("Failed to save test_output/debug_ridged_noise.png");
    }

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
        let chunk_origin = generate_chunk(0, 0, 0);
        let chunk_x = generate_chunk(1, 0, 0);
        let chunk_z = generate_chunk(0, 0, 1);
        assert!(chunk_origin.amount != chunk_x.amount);
        assert!(chunk_origin.amount != chunk_z.amount);

        let chunk_above = generate_chunk(0, 1, 0);
        assert!(chunk_origin.amount != chunk_above.amount);
    }
}

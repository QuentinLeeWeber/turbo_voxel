const PERLIN_SEED: u32 = 123456789;
const PERLIN_CHUNK_WIDTH: i32 = 4;

use super::{CHUNK_WIDTH, Chunk, Material};

pub fn generate_chunk(x: i32, y: i32, z: i32) -> Chunk {
    let mut chunk = Chunk {
        pos: [x, y, z],
        materials: [[[Material::default(); CHUNK_WIDTH]; CHUNK_WIDTH]; CHUNK_WIDTH],
        amount: [[[0.0f32; CHUNK_WIDTH]; CHUNK_WIDTH]; CHUNK_WIDTH],
    };

    //use image::{ImageBuffer, Rgb};
    //let mut img = ImageBuffer::new(CHUNK_WIDTH as u32, CHUNK_WIDTH as u32);

    /*for x_in in 0..CHUNK_WIDTH {
        for y_in in 0..CHUNK_WIDTH {
            let gray = (ridged_noise(x as f32, y as f32, x_in as i32, y_in as i32, 2, 0.5, 1.0)
                + 1.0)
                / 2.0
                * 255.0;
            img.put_pixel(
                x_in as u32,
                y_in as u32,
                Rgb([gray as u8, gray as u8, gray as u8]),
            );
        }
    }

    img.save("yeet.png").unwrap();*/

    let sea_level = 8.0;
    let mut terrain = ridged_noise(x, y);

    for x in 0..CHUNK_WIDTH {
        for y in 0..CHUNK_WIDTH {
            terrain[x][y] += sea_level;

            for z in 0..CHUNK_WIDTH {
                let diff = z as f32 - terrain[x][y];
                if diff > 1.0 {
                    chunk.materials[x][y][z] = Material::default();
                    chunk.amount[x][y][z] = 0.0;
                } else if diff > 0.0 {
                    chunk.materials[x][y][z] = Material::default();
                    chunk.amount[x][y][z] = diff;
                } else {
                    chunk.materials[x][y][z] = Material::default();
                    chunk.amount[x][y][z] = 1.0;
                }
            }
        }
    }

    chunk
}

fn ridged_noise(offset_x: i32, offset_y: i32) -> [[f32; CHUNK_WIDTH]; CHUNK_WIDTH] {
    let octaves = 2;
    let lacunarity = 0.5;
    let gain = 1.0;
    let mut map = [[0f32; CHUNK_WIDTH]; CHUNK_WIDTH];

    let perlin_noise = PerlinNoise::new(PERLIN_SEED, offset_x, offset_y);

    for x in 0..CHUNK_WIDTH {
        for y in 0..CHUNK_WIDTH {
            let mut amplitude = 1.0;
            let mut frequency = 1.0;
            let mut noise_sum = 0.0;
            let mut weight = 1.0;

            for _ in 0..octaves {
                let mut signal = perlin_noise
                    .noise((x as f32 * frequency) as i32, (y as f32 * frequency) as i32);
                signal = 1.0 - signal.abs();

                signal *= weight;
                weight = signal.clamp(0.0, 1.0);

                noise_sum += signal * amplitude;
                amplitude *= gain;
                frequency *= lacunarity;
            }

            map[x][y] = noise_sum;
        }
    }

    map
}

struct PerlinNoise {
    grads: [[[f32; 2]; CHUNK_WIDTH]; CHUNK_WIDTH],
}

impl PerlinNoise {
    fn new(seed: u32, offset_x: i32, offset_y: i32) -> Self {
        let mut grads = [[[0f32; 2]; CHUNK_WIDTH]; CHUNK_WIDTH];
        for x in 0..CHUNK_WIDTH {
            for y in 0..CHUNK_WIDTH {
                grads[x][y] = random_grad(x as i32 + offset_x, y as i32 + offset_y, seed);
            }
        }

        println!("grads: {:?}", grads);

        Self { grads }
    }

    fn noise(&self, x: i32, y: i32) -> f32 {
        let x0 = floor_div(x, PERLIN_CHUNK_WIDTH);
        let y0 = floor_div(y, PERLIN_CHUNK_WIDTH);
        let x1 = x0 + 1;
        let y1 = y0 + 1;

        let g00 = self.grads[x0 as usize][y0 as usize];
        let g10 = self.grads[x1 as usize][y0 as usize];
        let g01 = self.grads[x0 as usize][y1 as usize];
        let g11 = self.grads[x1 as usize][y1 as usize];

        let fx = floor_mod(x, PERLIN_CHUNK_WIDTH) as f32 / PERLIN_CHUNK_WIDTH as f32;
        let fy = floor_mod(y, PERLIN_CHUNK_WIDTH) as f32 / PERLIN_CHUNK_WIDTH as f32;

        let d00 = [fx, fy];
        let d10 = [fx - 1.0, fy];
        let d01 = [fx, fy - 1.0];
        let d11 = [fx - 1.0, fy - 1.0];

        let n00 = dot(d00, g00);
        let n10 = dot(d10, g10);
        let n01 = dot(d01, g01);
        let n11 = dot(d11, g11);

        let sx = fade(fx);
        let sy = fade(fy);

        let nx1 = interpolate(n00, n10, sx);
        let nx2 = interpolate(n01, n11, sx);

        interpolate(nx1, nx2, sy)
    }
}

fn floor_div(a: i32, b: i32) -> i32 {
    let d = a / b;
    let r = a % b;
    if r < 0 { d - 1 } else { d }
}

fn floor_mod(a: i32, b: i32) -> i32 {
    let r = a % b;
    if r < 0 { r + b } else { r }
}

fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn interpolate(a: f32, b: f32, t: f32) -> f32 {
    a + t * (b - a)
}

fn dot(v: [f32; 2], w: [f32; 2]) -> f32 {
    v[0] * w[0] + v[1] * w[1]
}

fn random_grad(x: i32, y: i32, seed: u32) -> [f32; 2] {
    let mut h = seed.wrapping_add(x as u32).wrapping_mul(0x85ebca6b);
    h = h.wrapping_add(y as u32).wrapping_mul(0xc2b2ae35);
    h ^= h >> 16;
    h = h.wrapping_mul(0x85ebca6b);

    let t = h as f32 / u32::MAX as f32;

    let angle = t * 2.0 * std::f32::consts::PI;
    [angle.cos(), angle.sin()]
}

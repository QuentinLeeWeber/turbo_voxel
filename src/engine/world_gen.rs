const PERLIN_SEED: u32 = 123456789;

use super::{CHUNK_WIDTH, Chunk, Material};

fn alloc_amount() -> Box<[[[f32; CHUNK_WIDTH]; CHUNK_WIDTH]; CHUNK_WIDTH]> {
    unsafe {
        let layout = std::alloc::Layout::new::<[[[f32; CHUNK_WIDTH]; CHUNK_WIDTH]; CHUNK_WIDTH]>();
        let ptr = std::alloc::alloc_zeroed(layout)
            as *mut [[[f32; CHUNK_WIDTH]; CHUNK_WIDTH]; CHUNK_WIDTH];
        Box::from_raw(ptr)
    }
}

fn alloc_materials() -> Box<[[[Material; CHUNK_WIDTH]; CHUNK_WIDTH]; CHUNK_WIDTH]> {
    unsafe {
        let layout =
            std::alloc::Layout::new::<[[[Material; CHUNK_WIDTH]; CHUNK_WIDTH]; CHUNK_WIDTH]>();
        let ptr = std::alloc::alloc_zeroed(layout)
            as *mut [[[Material; CHUNK_WIDTH]; CHUNK_WIDTH]; CHUNK_WIDTH];
        Box::from_raw(ptr)
    }
}

pub fn generate_chunk(x: i32, y: i32, z: i32) -> Chunk {
    println!("generate chunks");

    let mut chunk = Chunk {
        pos: [x, y, z],
        materials: alloc_materials(),
        amount: alloc_amount(),
    };

    let sea_level = 8.0;
    let mut terrain = ridged_noise(x, y, NoiseParams::default());

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

pub struct NoiseParams {
    pub amplitude: f32,
    pub gradient_spacing: i32,
}

impl Default for NoiseParams {
    fn default() -> Self {
        Self {
            amplitude: 40.0,
            gradient_spacing: 100,
        }
    }
}

fn ridged_noise(
    offset_x: i32,
    offset_y: i32,
    params: NoiseParams,
) -> Box<[[f32; CHUNK_WIDTH]; CHUNK_WIDTH]> {
    let octaves = 2;
    let lacunarity = 0.5;
    let gain = 1.0;

    let mut map = unsafe {
        let layout = std::alloc::Layout::new::<[[f32; CHUNK_WIDTH]; CHUNK_WIDTH]>();
        let ptr = std::alloc::alloc_zeroed(layout) as *mut [[f32; CHUNK_WIDTH]; CHUNK_WIDTH];
        Box::from_raw(ptr)
    };

    let perlin_noise = PerlinNoise::new(PERLIN_SEED, offset_x, offset_y, params.gradient_spacing);

    for x in 0..CHUNK_WIDTH {
        for y in 0..CHUNK_WIDTH {
            let mut amplitude = params.amplitude;
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
    grads: Vec<Vec<[f32; 2]>>,
    gradient_spacing: i32,
}

impl PerlinNoise {
    fn new(seed: u32, offset_x: i32, offset_y: i32, gradient_spacing: i32) -> Self {
        let grad_size = (CHUNK_WIDTH as i32 / gradient_spacing + 2) as usize;

        let mut grads = vec![vec![[0.0f32; 2]; grad_size]; grad_size];

        for x in 0..grad_size {
            for y in 0..grad_size {
                let grid_cells = CHUNK_WIDTH as i32 / gradient_spacing;
                grads[x][y] = random_grad(
                    x as i32 + offset_x * grid_cells,
                    y as i32 + offset_y * grid_cells,
                    seed,
                );
            }
        }

        Self {
            grads,
            gradient_spacing,
        }
    }

    fn noise(&self, x: i32, y: i32) -> f32 {
        let x0 = floor_div(x, self.gradient_spacing);
        let y0 = floor_div(y, self.gradient_spacing);
        let x1 = x0 + 1;
        let y1 = y0 + 1;

        let g00 = self.grads[x0 as usize][y0 as usize];
        let g10 = self.grads[x1 as usize][y0 as usize];
        let g01 = self.grads[x0 as usize][y1 as usize];
        let g11 = self.grads[x1 as usize][y1 as usize];

        let fx = floor_mod(x, self.gradient_spacing) as f32 / self.gradient_spacing as f32;
        let fy = floor_mod(y, self.gradient_spacing) as f32 / self.gradient_spacing as f32;

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

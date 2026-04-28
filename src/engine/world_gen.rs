const PERLIN_SEED: u32 = 123456789;

use super::{CHUNK_WIDTH, Chunk, Material, perlin_noise::PerlinNoise};

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

fn gen_2d_range(
    from1: usize,
    to1: usize,
    from2: usize,
    to2: usize,
) -> impl Iterator<Item = (usize, usize)> {
    (from1..to1).flat_map(move |a| (from2..to2).map(move |b| (a, b)))
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

    let perlin_noise = PerlinNoise::new(
        PERLIN_SEED,
        offset_x,
        offset_y,
        params.gradient_spacing,
        CHUNK_WIDTH as i32,
    );

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

pub(crate) struct PerlinNoise {
    grads: Vec<Vec<[f32; 2]>>,
    gradient_spacing: i32,
}

impl PerlinNoise {
    pub(crate) fn new(
        seed: u32,
        offset_x: i32,
        offset_y: i32,
        gradient_spacing: i32,
        chunk_width: i32,
    ) -> Self {
        let grad_size = (chunk_width as i32 / gradient_spacing + 2) as usize;

        let mut grads = vec![vec![[0.0f32; 2]; grad_size]; grad_size];

        for x in 0..grad_size {
            for y in 0..grad_size {
                let grid_cells = chunk_width as i32 / gradient_spacing;
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

    pub(crate) fn noise(&self, x: i32, y: i32) -> f32 {
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

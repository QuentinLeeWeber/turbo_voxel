pub(crate) struct PerlinNoise {
    grads: Vec<Vec<[f32; 2]>>,
    gradient_spacing: u32,
}

pub(crate) struct PerlinNoiseParams {
    pub seed: u32,
    pub chunk_x: i32,
    pub chunk_y: i32,
    pub gradient_spacing: u32,
    pub chunk_width: u32,
}

impl PerlinNoise {
    pub(crate) fn new(params: PerlinNoiseParams) -> Self {
        let grad_size = (params.chunk_width as i32 / params.gradient_spacing as i32 + 2) as usize;

        let mut grads = vec![vec![[0.0f32; 2]; grad_size]; grad_size];

        for x in 0..grad_size {
            for y in 0..grad_size {
                let grid_cells = params.chunk_width as i32 / params.gradient_spacing as i32;
                grads[x][y] = random_grad(
                    x as i32 + params.chunk_x * grid_cells,
                    y as i32 + params.chunk_y * grid_cells,
                    params.seed,
                );
            }
        }

        Self {
            grads,
            gradient_spacing: params.gradient_spacing,
        }
    }

    pub(crate) fn noise(&self, x: i32, y: i32) -> f32 {
        let x0 = floor_div(x, self.gradient_spacing) as usize;
        let y0 = floor_div(y, self.gradient_spacing) as usize;
        let x1 = x0 + 1;
        let y1 = y0 + 1;

        let g00 = self.grads[x0][y0];
        let g10 = self.grads[x1][y0];
        let g01 = self.grads[x0][y1];
        let g11 = self.grads[x1][y1];

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

fn floor_div(a: i32, b: u32) -> i32 {
    let d = a / b as i32;
    let r = a % b as i32;
    if r < 0 { d - 1 } else { d }
}

fn floor_mod(a: i32, b: u32) -> i32 {
    let r = a % b as i32;
    if r < 0 { r + b as i32 } else { r }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_floor_div() {
        assert_eq!(floor_div(9, 3), 3);
        assert_eq!(floor_div(10, 3), 3);
        assert_eq!(floor_div(-9, 3), -3);
        assert_eq!(floor_div(-10, 3), -4);
        assert_eq!(floor_div(0, 4), 0);
    }

    #[test]
    fn test_floor_mod() {
        assert_eq!(floor_mod(9, 3), 0);
        assert_eq!(floor_mod(10, 3), 1);
        assert_eq!(floor_mod(-9, 3), 0);
        assert_eq!(floor_mod(-10, 3), 2);
        assert_eq!(floor_mod(0, 4), 0);
    }

    #[test]
    fn test_interpolate() {
        assert_eq!(interpolate(0.0, 1.0, 0.5), 0.5);
        assert_eq!(interpolate(0.0, 2.0, 0.5), 1.0);
        assert_eq!(interpolate(0.0, 2.0, 0.25), 0.5);
    }

    #[test]
    fn test_dot() {
        assert_eq!(dot([1.0, 0.0], [0.0, 1.0]), 0.0);
        assert_eq!(dot([1.0, 0.0], [1.0, 0.0]), 1.0);
        assert_eq!(dot([1.0, 1.0], [1.0, 1.0]), 2.0);
        assert_eq!(dot([0.0, 0.0], [1.0, 0.0]), 0.0);
    }

    #[test]
    fn test_chunk_boundary_continuity() {
        let seed = 42;
        let gradient_spacing = 16;
        let chunk_width = 64;
        let w = chunk_width as i32;
        let eps = 1e-5;

        let new_noise = |ox, oy| {
            PerlinNoise::new(PerlinNoiseParams {
                seed,
                chunk_x: ox,
                chunk_y: oy,
                gradient_spacing,
                chunk_width,
            })
        };

        let n00 = new_noise(0, 0);
        let n10 = new_noise(1, 0);
        let n01 = new_noise(0, 1);
        let n11 = new_noise(1, 1);

        for y in 0..=w {
            let a = n00.noise(w, y);
            let b = n10.noise(0, y);
            assert!((a - b).abs() < eps,);
        }

        for x in 0..=w {
            let a = n00.noise(x, w);
            let b = n01.noise(x, 0);
            assert!((a - b).abs() < eps,);
        }

        let corner = [
            n00.noise(w, w),
            n10.noise(0, w),
            n01.noise(w, 0),
            n11.noise(0, 0),
        ];
        for &val in &corner[1..] {
            assert!((corner[0] - val).abs() < eps,);
        }
    }

    #[test]
    fn test_chunk_boundary_continuity_2() {
        let seed = 123456;
        let gradient_spacing = 16;
        let chunk_width = 64;
        let w = chunk_width as i32;
        let eps = 1e-5;

        let new_noise = |ox, oy| {
            PerlinNoise::new(PerlinNoiseParams {
                seed,
                chunk_x: ox,
                chunk_y: oy,
                gradient_spacing,
                chunk_width,
            })
        };

        let n00 = new_noise(0, 0);
        let n10 = new_noise(-1, 0);
        let n01 = new_noise(0, -1);
        let n11 = new_noise(-1, -1);

        for y in 0..=w {
            let a = n00.noise(0, y);
            let b = n10.noise(w, y);
            assert!((a - b).abs() < eps,);
        }

        for x in 0..=w {
            let a = n00.noise(x, 0);
            let b = n01.noise(x, w);
            assert!((a - b).abs() < eps,);
        }

        let corner = [
            n00.noise(0, 0),
            n10.noise(w, 0),
            n01.noise(0, w),
            n11.noise(w, w),
        ];
        for &val in &corner[1..] {
            assert!((corner[0] - val).abs() < eps,);
        }
    }
}

use crate::{game_object::Transform, physics::CoordinateBorders};

#[derive(Debug, Default, Clone, Copy)]
pub enum HitBox {
    #[default]
    None,
    Sphere {
        transform: Transform,
        radius: f32,
    },
    Cube {
        transform: Transform,
        size: f32,
    },
    Triangle {
        point1: [f32; 3],
        point2: [f32; 3],
        point3: [f32; 3],
    },
    Line {
        point: [f32; 3],
        vec: [f32; 3],
    },
}

impl HitBox {
    pub fn get_bounding_box(&self) -> Option<BoundingBox> {
        match self {
            HitBox::None => None,
            HitBox::Sphere { transform, radius } => Some(BoundingBox {
                x: CoordinateBorders::new(transform.pos[0] - radius, transform.pos[0] + radius),
                y: CoordinateBorders::new(transform.pos[1] - radius, transform.pos[1] + radius),
                z: CoordinateBorders::new(transform.pos[2] - radius, transform.pos[2] + radius),
            }),
            HitBox::Cube { transform, size } => {
                let max_dist = (3.0f32).sqrt() / 2. * size;
                Some(BoundingBox {
                    x: CoordinateBorders::new(
                        transform.pos[0] - max_dist,
                        transform.pos[0] + max_dist,
                    ),
                    y: CoordinateBorders::new(
                        transform.pos[1] - max_dist,
                        transform.pos[1] + max_dist,
                    ),
                    z: CoordinateBorders::new(
                        transform.pos[2] - max_dist,
                        transform.pos[2] + max_dist,
                    ),
                })
            }
            HitBox::Triangle {
                point1,
                point2,
                point3,
            } => {
                let min_x = point1[0].min(point2[0].min(point3[0]));
                let max_x = point1[0].max(point2[0].max(point3[0]));
                let min_y = point1[1].min(point2[1].min(point3[1]));
                let max_y = point1[1].max(point2[1].max(point3[1]));
                let min_z = point1[2].min(point2[2].min(point3[2]));
                let max_z = point1[2].max(point2[2].max(point3[2]));
                Some(BoundingBox {
                    x: CoordinateBorders::new(min_x, max_x),
                    y: CoordinateBorders::new(min_y, max_y),
                    z: CoordinateBorders::new(min_z, max_z),
                })
            }
            HitBox::Line { point, vec } => Some(BoundingBox {
                x: CoordinateBorders::new(
                    point[0].min(point[0] + vec[0]),
                    point[0].max(point[0] + vec[0]),
                ),
                y: CoordinateBorders::new(
                    point[1].min(point[1] + vec[1]),
                    point[1].max(point[1] + vec[1]),
                ),
                z: CoordinateBorders::new(
                    point[2].min(point[2] + vec[2]),
                    point[2].max(point[2] + vec[2]),
                ),
            }),
        }
    }
}

pub struct BoundingBox {
    pub x: CoordinateBorders,
    pub y: CoordinateBorders,
    pub z: CoordinateBorders,
}

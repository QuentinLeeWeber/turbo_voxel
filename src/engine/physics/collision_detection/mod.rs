use super::HitBox;
use cgmath::{InnerSpace, SquareMatrix, Vector3};

pub fn check_collision(obj_1: &HitBox, obj_2: &HitBox) -> bool {
    return true;
}

fn check_line_triangle_collision(line: &HitBox, triangle: &HitBox) -> bool {
    let (triangle_point, triangle_vec1, triangle_vec2) = {
        if let Some((p, v1, v2)) = extract_triangle_data(triangle) {
            (p, v1, v2)
        } else {
            return false;
        }
    };
    let (line_point, line_vec) = {
        if let Some((p, v)) = extract_line_data(line) {
            (p, v)
        } else {
            return false;
        }
    };
    let matrix = cgmath::Matrix3::from_cols(triangle_vec1, triangle_vec2, line_vec);
    let inv_matrix = matrix.invert();
    if inv_matrix.is_none() {
        return false;
    }
    let inv_matrix = inv_matrix.unwrap();
    let b = triangle_point - line_point;
    let (beta, gamma, alpha) = {
        let res: Vector3<f32> = inv_matrix * b;
        (-res[0], -res[1], res[2])
    };

    return beta >= 0.
        && beta <= 1.
        && gamma >= 0.
        && gamma <= 1.
        && alpha >= 0.
        && alpha <= 1.
        && beta + gamma <= 1.;
}

fn extract_triangle_data(triangle: &HitBox) -> Option<(Vector3<f32>, Vector3<f32>, Vector3<f32>)> {
    if !matches!(triangle, HitBox::Triangle { .. }) {
        return None;
    }
    //Linear Equation matrix*x = b
    let (point1, point2, point3) = {
        if let HitBox::Triangle {
            point1,
            point2,
            point3,
        } = triangle
        {
            (point1, point2, point3)
        } else {
            unreachable!("kein dreieck obwohl gecheckt");
        }
    };
    let triangle_point = Vector3::new(point1[0], point1[1], point1[2]);

    let triangle_vec1: Vector3<f32> =
        Vector3::new(point2[0], point2[1], point2[2]) - triangle_point;
    let triangle_vec2: Vector3<f32> =
        Vector3::new(point3[0], point3[1], point3[2]) - triangle_point;
    return Some((triangle_point, triangle_vec1, triangle_vec2));
}

fn extract_line_data(line: &HitBox) -> Option<(Vector3<f32>, Vector3<f32>)> {
    if !matches!(line, HitBox::Line { .. }) {
        return None;
    }
    let (point, vec) = {
        if let HitBox::Line { point, vec } = line {
            (point, vec)
        } else {
            unreachable!("keine linie obwohl gecheckt");
        }
    };
    let line_point = Vector3::new(point[0], point[1], point[2]);
    let line_vec = Vector3::new(vec[0], vec[1], vec[2]);
    return Some((line_point, line_vec));
}

fn extract_sphere_data(sphere: &HitBox) -> Option<(Vector3<f32>, f32)> {
    if !matches!(sphere, HitBox::Sphere { .. }) {
        return None;
    }
    let (transform, radius) = {
        if let HitBox::Sphere { transform, radius } = sphere {
            (transform, radius)
        } else {
            unreachable!("kein dreieck obwohl gecheckt");
        }
    };
    let sphere_point = Vector3::new(transform.pos[0], transform.pos[1], transform.pos[2]);
    Some((sphere_point, *radius))
}

fn check_line_sphere_collision(line: &HitBox, sphere: &HitBox) -> bool {
    let (line_point, line_vec) = {
        if let Some((p, v)) = extract_line_data(line) {
            (p, v)
        } else {
            return false;
        }
    };
    let (sphere_point, radius) = {
        if let Some((p, r)) = extract_sphere_data(line) {
            (p, r)
        } else {
            return false;
        }
    };
    let alpha = line_vec.dot(line_point - sphere_point) / line_vec.dot(line_vec) * -1.;
    let distance = sphere_point - (line_point + alpha * line_vec);
    let distance = distance.dot(distance);
    return distance < radius * radius;
}

pub fn get_collision_direction(stationary: &HitBox, moved: &HitBox) -> Vector3<f32> {
    return Vector3 {
        x: 0.0,
        y: 1.0,
        z: 0.0,
    };
}

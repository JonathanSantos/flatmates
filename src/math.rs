//! Vetores, fecho convexo e cinemática inversa de dois ossos.
use std::ops::{Add, Mul, Neg, Sub};

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct V2 {
    pub x: f64,
    pub y: f64,
}

pub const fn v2(x: f64, y: f64) -> V2 {
    V2 { x, y }
}

impl V2 {
    pub fn len(self) -> f64 {
        self.x.hypot(self.y)
    }
}

impl Add for V2 {
    type Output = V2;
    fn add(self, o: V2) -> V2 {
        v2(self.x + o.x, self.y + o.y)
    }
}
impl Sub for V2 {
    type Output = V2;
    fn sub(self, o: V2) -> V2 {
        v2(self.x - o.x, self.y - o.y)
    }
}
impl Mul<f64> for V2 {
    type Output = V2;
    fn mul(self, k: f64) -> V2 {
        v2(self.x * k, self.y * k)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct V3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

pub const fn v3(x: f64, y: f64, z: f64) -> V3 {
    V3 { x, y, z }
}

pub const UP: V3 = v3(0.0, 1.0, 0.0);

impl V3 {
    pub fn dot(self, o: V3) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    pub fn cross(self, o: V3) -> V3 {
        v3(self.y * o.z - self.z * o.y, self.z * o.x - self.x * o.z, self.x * o.y - self.y * o.x)
    }
    pub fn len(self) -> f64 {
        self.dot(self).sqrt()
    }
    pub fn norm(self) -> V3 {
        let l = self.len();
        if l < 1e-12 {
            return UP;
        }
        self * (1.0 / l)
    }
    pub fn lerp(self, o: V3, t: f64) -> V3 {
        self + (o - self) * t
    }
    /// Gira em torno do eixo Y: o vetor +Z vai para +X quando o ângulo cresce.
    pub fn rot_y(self, a: f64) -> V3 {
        let (s, c) = a.sin_cos();
        v3(self.x * c + self.z * s, self.y, -self.x * s + self.z * c)
    }
    /// Gira em torno do eixo X: o vetor +Y vai para +Z quando o ângulo cresce.
    pub fn rot_x(self, a: f64) -> V3 {
        let (s, c) = a.sin_cos();
        v3(self.x, self.y * c - self.z * s, self.y * s + self.z * c)
    }
    /// Gira em torno do eixo Z: o vetor +Y vai para +X quando o ângulo cresce.
    pub fn rot_z(self, a: f64) -> V3 {
        let (s, c) = a.sin_cos();
        v3(self.x * c + self.y * s, -self.x * s + self.y * c, self.z)
    }
}

impl Add for V3 {
    type Output = V3;
    fn add(self, o: V3) -> V3 {
        v3(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}
impl Sub for V3 {
    type Output = V3;
    fn sub(self, o: V3) -> V3 {
        v3(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}
impl Mul<f64> for V3 {
    type Output = V3;
    fn mul(self, k: f64) -> V3 {
        v3(self.x * k, self.y * k, self.z * k)
    }
}
impl Neg for V3 {
    type Output = V3;
    fn neg(self) -> V3 {
        v3(-self.x, -self.y, -self.z)
    }
}

/// Fecho convexo (cadeia monótona). Devolve os índices no sentido em que o
/// ângulo cresce de +X para +Y.
pub fn convex_hull(pts: &[V2]) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..pts.len()).collect();
    idx.sort_by(|&a, &b| (pts[a].x, pts[a].y).partial_cmp(&(pts[b].x, pts[b].y)).unwrap_or(std::cmp::Ordering::Equal));
    idx.dedup_by(|a, b| pts[*a] == pts[*b]);
    if idx.len() < 3 {
        return idx;
    }
    let cross =
        |o: usize, a: usize, b: usize| (pts[a].x - pts[o].x) * (pts[b].y - pts[o].y) - (pts[a].y - pts[o].y) * (pts[b].x - pts[o].x);
    let mut hull: Vec<usize> = Vec::with_capacity(idx.len() * 2);
    for &i in &idx {
        while hull.len() >= 2 && cross(hull[hull.len() - 2], hull[hull.len() - 1], i) <= 0.0 {
            hull.pop();
        }
        hull.push(i);
    }
    let lower = hull.len() + 1;
    for &i in idx.iter().rev().skip(1) {
        while hull.len() >= lower && cross(hull[hull.len() - 2], hull[hull.len() - 1], i) <= 0.0 {
            hull.pop();
        }
        hull.push(i);
    }
    hull.pop();
    hull
}

/// Resultado da cinemática inversa: junta do meio, ponta alcançada e se o
/// alvo estava fora do alcance.
pub struct Ik {
    pub mid: V3,
    pub end: V3,
    pub strained: bool,
}

/// Resolve um membro de dois ossos (`a` e `b`) da raiz até o alvo, dobrando
/// na direção de `pole`.
pub fn two_bone(root: V3, target: V3, a: f64, b: f64, pole: V3) -> Ik {
    let raw = (target - root).len();
    let max = (a + b) * 0.999;
    let min = (a - b).abs() + 1e-4;
    let d = raw.clamp(min, max);
    let dir = if raw < 1e-9 { -UP } else { (target - root) * (1.0 / raw) };
    let along = (a * a - b * b + d * d) / (2.0 * d);
    let height = (a * a - along * along).max(0.0).sqrt();
    let mut perp = pole - dir * pole.dot(dir);
    if perp.len() < 1e-6 {
        perp = v3(0.0, 0.0, 1.0) - dir * dir.z;
    }
    Ik { mid: root + dir * along + perp.norm() * height, end: root + dir * d, strained: raw > max + 0.02 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hull_of_square_with_inner_point_has_four_corners() {
        let pts = [v2(0.0, 0.0), v2(1.0, 0.0), v2(1.0, 1.0), v2(0.0, 1.0), v2(0.5, 0.5)];
        let h = convex_hull(&pts);
        assert_eq!(h.len(), 4);
        assert!(!h.contains(&4));
    }

    #[test]
    fn two_bone_keeps_bone_lengths() {
        let ik = two_bone(v3(0.0, 1.0, 0.0), v3(0.2, 0.3, 0.3), 0.42, 0.42, v3(0.0, 0.0, 1.0));
        assert!(((ik.mid - v3(0.0, 1.0, 0.0)).len() - 0.42).abs() < 1e-9);
        assert!(((ik.end - ik.mid).len() - 0.42).abs() < 1e-9);
        assert!(!ik.strained);
    }

    #[test]
    fn two_bone_flags_unreachable_target() {
        let ik = two_bone(v3(0.0, 0.0, 0.0), v3(5.0, 0.0, 0.0), 0.3, 0.3, UP);
        assert!(ik.strained);
        assert!((ik.end.x - 0.6).abs() < 0.01);
    }
}

//! Vetores, fecho convexo e cinemática inversa de dois ossos.
use std::f64::consts::{FRAC_2_PI, FRAC_PI_2, FRAC_PI_4, PI};
use std::ops::{Add, Mul, Neg, Sub};

/// Trigonometria determinística. `sin`, `cos` e `atan2` da biblioteca
/// padrão vêm da libm de cada sistema e podem diferir no último bit entre
/// macOS, Linux e Windows; isso bastava para trocar um desempate de ordem
/// e mudar o SVG. Aqui só entram soma, produto, divisão e raiz, que o IEEE
/// 754 define bit a bit, então a mesma cena dá os mesmos bytes em qualquer
/// plataforma.
pub trait Det {
    fn dsin(self) -> f64;
    fn dcos(self) -> f64;
    fn dsin_cos(self) -> (f64, f64);
    fn datan2(self, x: f64) -> f64;
    fn dhypot(self, other: f64) -> f64;
}

/// Reduz o ângulo a [-π/4, π/4] e diz em que quadrante ele estava.
fn reduce(x: f64) -> (f64, i64) {
    const HI: f64 = FRAC_PI_2;
    const LO: f64 = 6.123233995736766e-17;
    let k = (x * FRAC_2_PI).round();
    ((x - k * HI) - k * LO, (k as i64).rem_euclid(4))
}

fn sin_poly(r: f64) -> f64 {
    let r2 = r * r;
    let mut acc = 0.0;
    // Série de Taylor em Horner, até r^17.
    for n in (1..=8).rev() {
        acc = (1.0 - acc) * r2 / ((2 * n) as f64 * (2 * n + 1) as f64);
    }
    r * (1.0 - acc)
}

fn cos_poly(r: f64) -> f64 {
    let r2 = r * r;
    let mut acc = 0.0;
    for n in (1..=9).rev() {
        acc = (1.0 - acc) * r2 / ((2 * n - 1) as f64 * (2 * n) as f64);
    }
    1.0 - acc
}

/// Arco tangente para |z| <= 1.
fn atan_unit(z: f64) -> f64 {
    // tan(π/8): acima disso, gira π/4 para a série convergir rápido.
    let (t, base) = if z.abs() > 0.414_213_562_373_095_05 {
        let s = if z >= 0.0 { 1.0 } else { -1.0 };
        ((z.abs() - 1.0) / (z.abs() + 1.0) * s, FRAC_PI_4 * s)
    } else {
        (z, 0.0)
    };
    let t2 = t * t;
    let mut acc = 0.0;
    for n in (0..24).rev() {
        acc = 1.0 / (2 * n + 1) as f64 - t2 * acc;
    }
    base + t * acc
}

impl Det for f64 {
    fn dsin(self) -> f64 {
        self.dsin_cos().0
    }
    fn dcos(self) -> f64 {
        self.dsin_cos().1
    }
    fn dsin_cos(self) -> (f64, f64) {
        let (r, quadrant) = reduce(self);
        let (s, c) = (sin_poly(r), cos_poly(r));
        match quadrant {
            0 => (s, c),
            1 => (c, -s),
            2 => (-s, -c),
            _ => (-c, s),
        }
    }
    fn datan2(self, x: f64) -> f64 {
        let y = self;
        if x == 0.0 && y == 0.0 {
            return 0.0;
        }
        if y.abs() <= x.abs() {
            let a = atan_unit(y / x);
            return if x > 0.0 {
                a
            } else if y >= 0.0 {
                a + PI
            } else {
                a - PI
            };
        }
        let a = atan_unit(x / y);
        if y > 0.0 {
            FRAC_PI_2 - a
        } else {
            -FRAC_PI_2 - a
        }
    }
    fn dhypot(self, other: f64) -> f64 {
        (self * self + other * other).sqrt()
    }
}

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
        self.x.dhypot(self.y)
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
        let (s, c) = a.dsin_cos();
        v3(self.x * c + self.z * s, self.y, -self.x * s + self.z * c)
    }
    /// Gira em torno do eixo X: o vetor +Y vai para +Z quando o ângulo cresce.
    pub fn rot_x(self, a: f64) -> V3 {
        let (s, c) = a.dsin_cos();
        v3(self.x, self.y * c - self.z * s, self.y * s + self.z * c)
    }
    /// Gira em torno do eixo Z: o vetor +Y vai para +X quando o ângulo cresce.
    pub fn rot_z(self, a: f64) -> V3 {
        let (s, c) = a.dsin_cos();
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
    fn deterministic_trig_matches_the_standard_library() {
        for k in -400..=400 {
            let x = k as f64 * 0.0371;
            assert!((x.dsin() - x.sin()).abs() < 1e-13, "sin({x})");
            assert!((x.dcos() - x.cos()).abs() < 1e-13, "cos({x})");
        }
        for (y, x) in [(0.3, 1.0), (1.0, 0.3), (-0.7, 0.2), (0.5, -2.0), (-0.5, -2.0), (1.0, 0.0), (-1.0, 0.0), (0.0, -1.0), (2.0, 2.0)] {
            assert!((y.datan2(x) - f64::atan2(y, x)).abs() < 1e-13, "atan2({y}, {x})");
        }
        assert_eq!(3.0.dhypot(4.0), 5.0);
    }

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

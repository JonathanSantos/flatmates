//! Primitivas de desenho: geometria no mundo 3D que vira caminho SVG só no
//! fim, depois de passar pela câmera.
use crate::camera::Camera;
use crate::color::Rgb;
use crate::math::{convex_hull, v2, V2, V3};
use std::f64::consts::TAU;
use std::fmt::Write;

/// Câmera mais enquadramento: leva um ponto do mundo a um pixel.
pub struct Projector {
    pub cam: Camera,
    pub scale: f64,
    pub ox: f64,
    pub oy: f64,
}

impl Projector {
    pub fn p(&self, w: V3) -> V2 {
        let v = self.cam.view(w);
        v2(self.ox + v.x * self.scale, self.oy - v.y * self.scale)
    }
}

#[derive(Clone, Debug)]
pub enum Geom {
    /// Fecho convexo de esferas: a silhueta vale de qualquer ângulo.
    Hull(Vec<(V3, f64)>),
    /// União dos fechos de cada par consecutivo de esferas (tubo que afina).
    Chain(Vec<(V3, f64)>),
    /// União de vários fechos: permite cintura, que um fecho só não tem.
    Union(Vec<Vec<(V3, f64)>>),
    /// Polígono plano; `round` arredonda os cantos (em metros).
    Poly { pts: Vec<V3>, round: f64 },
    /// Fecho convexo de pontos soltos (lateral de cilindro).
    HullPts(Vec<V3>),
    /// Traço com pontas redondas; `smooth` passa uma curva pelos pontos.
    Curve { pts: Vec<V3>, width: f64, smooth: bool },
    /// Parte visível de uma calota esférica (cabelo, barba, rosto).
    Cap { c: V3, r: f64, axis: V3, angle: f64 },
}

#[derive(Clone, Debug)]
pub struct Shape {
    pub geom: Geom,
    pub fill: Rgb,
    pub opacity: f64,
    /// Sombra tonal: uma faixa mais escura no lado direito da forma.
    pub shade: bool,
}

impl Shape {
    pub fn new(geom: Geom, fill: Rgb) -> Shape {
        Shape { geom, fill, opacity: 1.0, shade: false }
    }
    pub fn hull(discs: &[(V3, f64)], fill: Rgb) -> Shape {
        Shape::new(Geom::Hull(discs.to_vec()), fill)
    }
    pub fn union(parts: Vec<Vec<(V3, f64)>>, fill: Rgb) -> Shape {
        Shape::new(Geom::Union(parts), fill)
    }
    pub fn chain(discs: &[(V3, f64)], fill: Rgb) -> Shape {
        Shape::new(Geom::Chain(discs.to_vec()), fill)
    }
    /// Cadeia com a linha central suavizada (Catmull-Rom): rabos, mechas e folhas.
    pub fn chain_smooth(discs: &[(V3, f64)], fill: Rgb) -> Shape {
        const SUB: usize = 5;
        let n = discs.len() as isize;
        let at = |i: isize| discs[i.clamp(0, n - 1) as usize];
        let mut out = Vec::new();
        for i in 0..n - 1 {
            let (p0, p1, p2, p3) = (at(i - 1).0, at(i).0, at(i + 1).0, at(i + 2).0);
            for k in 0..SUB {
                let t = k as f64 / SUB as f64;
                let (t2, t3) = (t * t, t * t * t);
                let p =
                    (p1 * 2.0 + (p2 - p0) * t + (p0 * 2.0 - p1 * 5.0 + p2 * 4.0 - p3) * t2 + (p1 * 3.0 - p0 - p2 * 3.0 + p3) * t3) * 0.5;
                out.push((p, at(i).1 + (at(i + 1).1 - at(i).1) * t));
            }
        }
        out.push(at(n - 1));
        Shape::new(Geom::Chain(out), fill)
    }
    pub fn poly(pts: &[V3], round: f64, fill: Rgb) -> Shape {
        Shape::new(Geom::Poly { pts: pts.to_vec(), round }, fill)
    }
    pub fn curve(pts: &[V3], width: f64, smooth: bool, fill: Rgb) -> Shape {
        Shape::new(Geom::Curve { pts: pts.to_vec(), width, smooth }, fill)
    }
    pub fn cap(c: V3, r: f64, axis: V3, angle_deg: f64, fill: Rgb) -> Shape {
        Shape::new(Geom::Cap { c, r, axis: axis.norm(), angle: angle_deg.to_radians() }, fill)
    }
    pub fn shaded(mut self, on: bool) -> Shape {
        self.shade = on;
        self
    }
    pub fn alpha(mut self, a: f64) -> Shape {
        self.opacity = a;
        self
    }
}

/// Uma peça da cena: formas desenhadas juntas, mais o que a ordenação
/// precisa saber sobre onde ela está.
#[derive(Clone, Debug)]
pub struct Prim {
    pub shapes: Vec<Shape>,
    /// Pontos representativos no mundo (vértices da face ou centros das esferas).
    pub key: Vec<V3>,
    /// Normal no mundo, quando a peça é uma face plana.
    pub normal: Option<V3>,
    /// Empurrão de profundidade (metros) usado só no desempate.
    pub bias: f64,
    /// Plano (ponto, normal) acima do qual a peça não tem nada: a lateral de
    /// um cilindro fica toda abaixo do tampo.
    pub ceiling: Option<(V3, V3)>,
}

impl Prim {
    pub fn blob(shapes: Vec<Shape>, key: &[V3], bias: f64) -> Prim {
        Prim { shapes, key: key.to_vec(), normal: None, bias, ceiling: None }
    }
    pub fn face(shapes: Vec<Shape>, key: &[V3], normal: V3) -> Prim {
        Prim { shapes, key: key.to_vec(), normal: Some(normal), bias: 0.0, ceiling: None }
    }

    /// Contorno convexo em espaço de vista, para testar sobreposição real.
    pub fn outline(&self, cam: &Camera) -> Vec<V2> {
        let mut pts: Vec<V2> = Vec::new();
        let mut ring = |p: V3, r: f64| {
            let v = cam.view(p);
            if r <= 1e-6 {
                pts.push(v2(v.x, v.y));
                return;
            }
            for k in 0..8 {
                let a = TAU * k as f64 / 8.0;
                // Octógono circunscrito: nunca fica menor que o disco.
                pts.push(v2(v.x + r * 1.083 * a.cos(), v.y + r * 1.083 * a.sin()));
            }
        };
        for s in &self.shapes {
            match &s.geom {
                Geom::Hull(d) | Geom::Chain(d) => d.iter().for_each(|&(c, r)| ring(c, r)),
                Geom::Union(parts) => parts.iter().flatten().for_each(|&(c, r)| ring(c, r)),
                Geom::Poly { pts, round } => pts.iter().for_each(|&p| ring(p, *round)),
                Geom::HullPts(pts) => pts.iter().for_each(|&p| ring(p, 0.0)),
                Geom::Curve { pts, width, .. } => pts.iter().for_each(|&p| ring(p, width / 2.0)),
                Geom::Cap { c, r, .. } => ring(*c, *r),
            }
        }
        convex_hull(&pts).into_iter().map(|i| pts[i]).collect()
    }

    /// Caixa em espaço de vista (metros): [min_x, min_y, max_x, max_y].
    pub fn bounds(&self, cam: &Camera) -> [f64; 4] {
        let mut b = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
        let mut add = |p: V3, r: f64| {
            let v = cam.view(p);
            b[0] = b[0].min(v.x - r);
            b[1] = b[1].min(v.y - r);
            b[2] = b[2].max(v.x + r);
            b[3] = b[3].max(v.y + r);
        };
        for s in &self.shapes {
            match &s.geom {
                Geom::Hull(d) | Geom::Chain(d) => d.iter().for_each(|&(c, r)| add(c, r)),
                Geom::Union(parts) => parts.iter().flatten().for_each(|&(c, r)| add(c, r)),
                Geom::Poly { pts, round } => pts.iter().for_each(|&p| add(p, *round)),
                Geom::HullPts(pts) => pts.iter().for_each(|&p| add(p, 0.0)),
                Geom::Curve { pts, width, .. } => pts.iter().for_each(|&p| add(p, width / 2.0)),
                Geom::Cap { c, r, .. } => add(*c, *r),
            }
        }
        b
    }
}

/// Número com uma casa decimal, sem zero à toa e sem "-0".
pub fn num(v: f64) -> String {
    let r = (v * 10.0).round() / 10.0;
    let r = if r == 0.0 { 0.0 } else { r };
    if r.fract() == 0.0 {
        format!("{}", r as i64)
    } else {
        format!("{r:.1}")
    }
}

fn pt(p: V2) -> String {
    format!("{} {}", num(p.x), num(p.y))
}

fn circle_path(c: V2, r: f64) -> String {
    format!("M{} {}a{r} {r} 0 1 0 {d} 0a{r} {r} 0 1 0 -{d} 0Z", num(c.x - r), num(c.y), r = num(r), d = num(2.0 * r))
}

const ARC_STEPS: usize = 48;

/// Onde começa a sombra tonal, em fração da largura da forma.
const SHADE_AT: f64 = 0.68;

fn span(discs: &[(V2, f64)]) -> f64 {
    let lo = discs.iter().map(|(c, r)| c.x - r).fold(f64::MAX, f64::min);
    let hi = discs.iter().map(|(c, r)| c.x + r).fold(f64::MIN, f64::max);
    hi - lo
}

/// Contorno do fecho convexo de discos, com arcos de verdade em cada disco.
pub fn hull_path(discs: &[(V2, f64)]) -> String {
    let mut ds: Vec<(V2, f64)> = Vec::with_capacity(discs.len());
    for (i, &(c, r)) in discs.iter().enumerate() {
        let swallowed =
            discs.iter().enumerate().any(|(j, &(c2, r2))| j != i && (c - c2).len() + r <= r2 + 1e-9 && (r2 > r + 1e-9 || j < i));
        if !swallowed {
            ds.push((c, r));
        }
    }
    if ds.is_empty() {
        return String::new();
    }
    if ds.len() == 1 {
        return circle_path(ds[0].0, ds[0].1);
    }
    let mut pts: Vec<V2> = Vec::new();
    let mut owner: Vec<usize> = Vec::new();
    for (i, &(c, r)) in ds.iter().enumerate() {
        if r < 0.05 {
            pts.push(c);
            owner.push(i);
            continue;
        }
        for k in 0..ARC_STEPS {
            let a = TAU * k as f64 / ARC_STEPS as f64;
            pts.push(v2(c.x + r * a.cos(), c.y + r * a.sin()));
            owner.push(i);
        }
    }
    let hull = convex_hull(&pts);
    if hull.len() < 3 {
        return String::new();
    }
    let n = hull.len();
    let Some(start) = (0..n).find(|&i| owner[hull[i]] != owner[hull[(i + n - 1) % n]]) else {
        let d = ds[owner[hull[0]]];
        return circle_path(d.0, d.1);
    };
    let mut out = format!("M{}", pt(pts[hull[start]]));
    let mut i = 0;
    while i < n {
        let first = hull[(start + i) % n];
        let mut len = 1;
        while i + len < n && owner[hull[(start + i + len) % n]] == owner[first] {
            len += 1;
        }
        if len > 1 {
            let last = hull[(start + i + len - 1) % n];
            let r = num(ds[owner[first]].1);
            let large = if len - 1 > ARC_STEPS / 2 { 1 } else { 0 };
            let _ = write!(out, "A{r} {r} 0 {large} 1 {}", pt(pts[last]));
        }
        i += len;
        if i < n {
            let _ = write!(out, "L{}", pt(pts[hull[(start + i) % n]]));
        }
    }
    out.push('Z');
    out
}

fn poly_path(pts: &[V2]) -> String {
    let mut out = String::new();
    for (i, p) in pts.iter().enumerate() {
        out.push(if i == 0 { 'M' } else { 'L' });
        out.push_str(&pt(*p));
    }
    out.push('Z');
    out
}

/// Curva suave (Catmull-Rom convertida em Bézier cúbica) pelos pontos.
fn smooth_path(pts: &[V2], closed: bool) -> String {
    let n = pts.len();
    if n < 3 {
        return open_path(pts);
    }
    let at = |i: isize| -> V2 {
        if closed {
            pts[i.rem_euclid(n as isize) as usize]
        } else {
            pts[i.clamp(0, n as isize - 1) as usize]
        }
    };
    let mut out = format!("M{}", pt(pts[0]));
    let segs = if closed { n } else { n - 1 };
    for i in 0..segs as isize {
        let (p0, p1, p2, p3) = (at(i - 1), at(i), at(i + 1), at(i + 2));
        let c1 = p1 + (p2 - p0) * (1.0 / 6.0);
        let c2 = p2 - (p3 - p1) * (1.0 / 6.0);
        let _ = write!(out, "C{} {} {}", pt(c1), pt(c2), pt(p2));
    }
    if closed {
        out.push('Z');
    }
    out
}

fn open_path(pts: &[V2]) -> String {
    let mut out = String::new();
    for (i, p) in pts.iter().enumerate() {
        out.push(if i == 0 { 'M' } else { 'L' });
        out.push_str(&pt(*p));
    }
    out
}

pub fn blob_path(pts: &[V2]) -> String {
    smooth_path(pts, true)
}

/// Região visível de uma calota de esfera unitária vista de +Z.
/// `a` é o eixo já em espaço de vista. Devolve os contornos e se o
/// preenchimento é par-ímpar (disco com furo).
pub fn cap_outline(a: V3, alpha: f64) -> (Vec<Vec<V2>>, bool) {
    const N: usize = 72;
    let (sa, ca) = alpha.sin_cos();
    let helper = if a.y.abs() < 0.9 { V3 { x: 0.0, y: 1.0, z: 0.0 } } else { V3 { x: 1.0, y: 0.0, z: 0.0 } };
    let e1 = a.cross(helper).norm();
    let e2 = a.cross(e1);
    let ring: Vec<V3> = (0..N)
        .map(|i| {
            let f = TAU * i as f64 / N as f64;
            a * ca + (e1 * f.cos() + e2 * f.sin()) * sa
        })
        .collect();
    let disc = || -> Vec<V2> {
        (0..N)
            .map(|i| {
                let f = TAU * i as f64 / N as f64;
                v2(f.cos(), f.sin())
            })
            .collect()
    };
    let visible = ring.iter().filter(|p| p.z >= 0.0).count();
    if visible == N {
        let ell: Vec<V2> = ring.iter().map(|p| v2(p.x, p.y)).collect();
        return if a.z >= 0.0 { (vec![ell], false) } else { (vec![disc(), ell], true) };
    }
    if visible == 0 {
        return if a.z >= ca { (vec![disc()], false) } else { (vec![], false) };
    }
    let start = (0..N).find(|&i| ring[i].z >= 0.0 && ring[(i + N - 1) % N].z < 0.0).unwrap_or(0);
    let on_rim = |hidden: V3, shown: V3| -> V2 {
        let t = hidden.z / (hidden.z - shown.z);
        let p = hidden.lerp(shown, t);
        let l = p.x.hypot(p.y).max(1e-9);
        v2(p.x / l, p.y / l)
    };
    let mut out: Vec<V2> = vec![on_rim(ring[(start + N - 1) % N], ring[start])];
    let mut i = start;
    while ring[i].z >= 0.0 {
        out.push(v2(ring[i].x, ring[i].y));
        i = (i + 1) % N;
    }
    let exit = on_rim(ring[i], ring[(i + N - 1) % N]);
    out.push(exit);
    // Fecha pela silhueta, do ponto de saída de volta ao de entrada, pelo
    // lado que pertence à calota.
    let (t0, t1) = (exit.y.atan2(exit.x), out[0].y.atan2(out[0].x));
    let ccw = (t1 - t0).rem_euclid(TAU);
    let mid = t0 + ccw / 2.0;
    let inside = a.x * mid.cos() + a.y * mid.sin() >= ca;
    let span = if inside { ccw } else { ccw - TAU };
    let steps = ((span.abs() / 0.09).ceil() as usize).max(1);
    for k in 1..steps {
        let t = t0 + span * k as f64 / steps as f64;
        out.push(v2(t.cos(), t.sin()));
    }
    (vec![out], false)
}

impl Shape {
    pub fn svg(&self, pj: &Projector) -> String {
        let s = pj.scale;
        let fill = self.fill.css();
        let mut extra = String::new();
        let mut shade_shift = None;
        let d = match &self.geom {
            Geom::Hull(discs) => {
                let px: Vec<(V2, f64)> = discs.iter().map(|&(c, r)| (pj.p(c), r * s)).collect();
                if self.shade {
                    shade_shift = Some(span(&px) * SHADE_AT);
                }
                hull_path(&px)
            }
            Geom::Union(parts) => {
                let px: Vec<Vec<(V2, f64)>> = parts.iter().map(|d| d.iter().map(|&(c, r)| (pj.p(c), r * s)).collect()).collect();
                if self.shade {
                    shade_shift = Some(span(&px.concat()) * SHADE_AT);
                }
                px.iter().map(|d| hull_path(d)).collect::<String>()
            }
            Geom::Chain(discs) => {
                let px: Vec<(V2, f64)> = discs.iter().map(|&(c, r)| (pj.p(c), r * s)).collect();
                px.windows(2).map(hull_path).collect::<String>()
            }
            Geom::Poly { pts, round } => {
                if *round > 0.0 {
                    extra = format!(" stroke=\"{fill}\" stroke-width=\"{}\" stroke-linejoin=\"round\"", num(2.0 * round * s));
                }
                poly_path(&pts.iter().map(|&p| pj.p(p)).collect::<Vec<_>>())
            }
            Geom::HullPts(pts) => {
                let px: Vec<V2> = pts.iter().map(|&p| pj.p(p)).collect();
                let hull: Vec<V2> = convex_hull(&px).into_iter().map(|i| px[i]).collect();
                poly_path(&hull)
            }
            Geom::Curve { pts, width, smooth } => {
                let px: Vec<V2> = pts.iter().map(|&p| pj.p(p)).collect();
                let d = if *smooth { smooth_path(&px, false) } else { open_path(&px) };
                let op = if self.opacity < 1.0 { format!(" opacity=\"{}\"", self.opacity) } else { String::new() };
                return format!(
                    "<path d=\"{d}\" fill=\"none\" stroke=\"{fill}\" stroke-width=\"{}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"{op}/>",
                    num((width * s).max(0.6))
                );
            }
            Geom::Cap { c, r, axis, angle } => {
                let center = pj.p(*c);
                let a = pj.cam.view(*axis);
                let (loops, evenodd) = cap_outline(a, *angle);
                if evenodd {
                    extra = " fill-rule=\"evenodd\"".into();
                }
                loops
                    .iter()
                    .map(|l| {
                        let px: Vec<V2> = l.iter().map(|u| v2(center.x + u.x * r * s, center.y - u.y * r * s)).collect();
                        poly_path(&px)
                    })
                    .collect::<String>()
            }
        };
        if d.is_empty() {
            return String::new();
        }
        if self.opacity < 1.0 {
            let _ = write!(extra, " opacity=\"{}\"", self.opacity);
        }
        let base = format!("<path d=\"{d}\" fill=\"{fill}\"{extra}/>");
        let Some(dx) = shade_shift else { return base };
        // A mesma forma, deslocada e recortada por ela mesma. O id vem do
        // próprio caminho, então a saída continua determinística.
        let id = d.bytes().fold(0x811c_9dc5_u32, |h, b| (h ^ b as u32).wrapping_mul(0x0100_0193));
        format!(
            "{base}<clipPath id=\"k{id:08x}\"><path d=\"{d}\"/></clipPath><g clip-path=\"url(#k{id:08x})\"><path d=\"{d}\" transform=\"translate({} {})\" fill=\"{}\"/></g>",
            num(dx),
            num(dx * 0.10),
            self.fill.mix(crate::color::INK, 0.075).css()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::v3;

    #[test]
    fn num_is_compact() {
        assert_eq!(num(12.0), "12");
        assert_eq!(num(12.34), "12.3");
        assert_eq!(num(-0.01), "0");
    }

    #[test]
    fn hull_of_two_discs_uses_two_arcs() {
        let d = hull_path(&[(v2(0.0, 0.0), 10.0), (v2(40.0, 0.0), 6.0)]);
        assert_eq!(d.matches('A').count(), 2, "{d}");
        assert!(d.ends_with('Z'));
    }

    #[test]
    fn swallowed_disc_becomes_single_circle() {
        let d = hull_path(&[(v2(0.0, 0.0), 10.0), (v2(1.0, 0.0), 3.0)]);
        assert_eq!(d.matches('a').count(), 2, "{d}");
    }

    #[test]
    fn cap_facing_camera_is_an_ellipse_and_cap_behind_is_hidden() {
        let (front, _) = cap_outline(v3(0.0, 0.0, 1.0), 1.0);
        assert_eq!(front.len(), 1);
        let (back, _) = cap_outline(v3(0.0, 0.0, -1.0), 1.0);
        assert!(back.is_empty());
    }

    #[test]
    fn cap_on_the_side_stays_inside_the_disc() {
        let (loops, _) = cap_outline(v3(1.0, 0.0, 0.0), 1.2);
        assert_eq!(loops.len(), 1);
        assert!(loops[0].iter().all(|p| p.len() <= 1.0 + 1e-6));
        // A calota virada para a direita cobre o lado direito do disco.
        assert!(loops[0].iter().any(|p| p.x > 0.95));
        assert!(loops[0].iter().all(|p| p.x > -0.1));
    }
}

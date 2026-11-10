//! Objetos da cena. Móveis são sólidos com três tons por cor; plantas e
//! animais usam as mesmas esferas varridas do corpo dos personagens.
use crate::camera::Camera;
use crate::color::{self, Rgb, Style, GREY, INK, LEAF, LEAF_DARK, LEAF_LIGHT, PAPER, SLATE, WHITE, WOOD};
use crate::math::{v3, UP, V3};
use crate::prim::{Geom, Prim, Shape};
use crate::rng::Rng;
use crate::spec::{CustomDef, Part};
use std::f64::consts::TAU;

pub const KINDS: [&str; 16] =
    ["desk", "chair", "stool", "sofa", "ball", "laptop", "mug", "books", "plant", "monstera", "board", "card", "box", "lamp", "cat", "dog"];
pub const CONTENTS: [&str; 4] = ["chart", "ui", "code", "blank"];
pub const HOLDS: [&str; 5] = ["phone", "mug", "book", "tablet", "box"];

/// Posição, giro e escala de um objeto no mundo.
#[derive(Clone, Copy, Debug)]
pub struct Xf {
    pub pos: V3,
    pub yaw: f64,
    pub k: f64,
}

impl Xf {
    pub fn at(&self, x: f64, y: f64, z: f64) -> V3 {
        self.pos + v3(x, y, z).rot_y(self.yaw) * self.k
    }
    pub fn p(&self, q: V3) -> V3 {
        self.at(q.x, q.y, q.z)
    }
}

#[derive(Default)]
pub struct Built {
    pub prims: Vec<Prim>,
    /// Pontos de contato no mundo: assentos, teclado.
    pub anchors: Vec<(String, V3)>,
    /// Altura local da superfície onde outros objetos podem ficar.
    pub top: f64,
    /// Meia-largura e meia-profundidade da sombra no chão.
    pub shadow: Option<(f64, f64)>,
}

/// Três tons por cor: claro em cima, base de frente, escuro na lateral direita.
fn toned(cam: &Camera, n: V3, c: Rgb) -> Rgb {
    if n.y > 0.7 {
        return c.light();
    }
    if n.y < -0.7 || cam.view(n).x > 0.35 {
        return c.shade();
    }
    c
}

/// Caixa com faces de costas descartadas. Ordem: +x, -x, +y, -y, +z, -z.
pub fn boxed(cam: &Camera, m: &dyn Fn(V3) -> V3, c: V3, size: V3, color: Rgb, round: f64) -> [Option<Prim>; 6] {
    let axes = [v3(1.0, 0.0, 0.0), UP, v3(0.0, 0.0, 1.0)];
    let half = [size.x / 2.0, size.y / 2.0, size.z / 2.0];
    let mut faces: [Option<Prim>; 6] = Default::default();
    for k in 0..3 {
        for (j, sign) in [1.0, -1.0].into_iter().enumerate() {
            let fc = c + axes[k] * (sign * half[k]);
            let n = (m(fc + axes[k] * sign) - m(fc)).norm();
            if cam.view(n).z <= 0.01 {
                continue;
            }
            let (a1, a2) = (axes[(k + 1) % 3] * half[(k + 1) % 3], axes[(k + 2) % 3] * half[(k + 2) % 3]);
            let pts = [fc + a1 + a2, fc + a1 - a2, fc - a1 - a2, fc - a1 + a2].map(m);
            faces[k * 2 + j] = Some(Prim::face(vec![Shape::poly(&pts, round, toned(cam, n, color))], &pts, n));
        }
    }
    faces
}

fn push_box(out: &mut Vec<Prim>, cam: &Camera, m: &dyn Fn(V3) -> V3, c: V3, size: V3, color: Rgb, round: f64) {
    out.extend(boxed(cam, m, c, size, color, round).into_iter().flatten());
}

/// Cilindro ou tronco de cone em pé: lateral em um tom, tampo mais claro.
#[allow(clippy::too_many_arguments)]
pub fn cylinder(out: &mut Vec<Prim>, cam: &Camera, m: &dyn Fn(V3) -> V3, base: V3, r0: f64, r1: f64, h: f64, color: Rgb) {
    const N: usize = 40;
    let ring = |r: f64, y: f64| -> Vec<V3> {
        (0..N)
            .map(|i| {
                let a = TAU * i as f64 / N as f64;
                m(base + v3(r * a.cos(), y, r * a.sin()))
            })
            .collect()
    };
    let top = ring(r1, h);
    let mut all = ring(r0, 0.0);
    all.extend(top.iter().copied());
    let mut side = Prim::blob(vec![Shape::new(Geom::HullPts(all), color)], &[m(base), m(base + UP * h)], 0.0);
    side.ceiling = Some((m(base + UP * h), UP));
    out.push(side);
    if cam.view(UP).z > 0.01 {
        let key: Vec<V3> = top.iter().step_by(N / 8).copied().collect();
        out.push(Prim::face(vec![Shape::poly(&top, 0.0, color.light())], &key, UP));
    }
}

/// Conteúdo de uma tela ou painel plano. `o` é o canto de cima à esquerda,
/// `u` vai até a direita e `v` até embaixo (vetores do mundo).
pub fn content(kind: &str, o: V3, u: V3, v: V3, accent: Rgb) -> Vec<Shape> {
    let (w, h) = (u.len(), v.len());
    let (un, vn) = (u.norm(), v.norm());
    let p = |a: f64, b: f64| o + u * a + v * b;
    let bar = |a: f64, b: f64, wa: f64, hb: f64, c: Rgb| {
        let r = (wa * w).min(hb * h) * 0.28;
        let (ra, rb) = (r / w, r / h);
        Shape::poly(&[p(a + ra, b + rb), p(a + wa - ra, b + rb), p(a + wa - ra, b + hb - rb), p(a + ra, b + hb - rb)], r, c)
    };
    let dot = |a: f64, b: f64, rad: f64, c: Rgb| {
        let pts: Vec<V3> = (0..24)
            .map(|i| {
                let t = TAU * i as f64 / 24.0;
                p(a, b) + un * (rad * t.cos()) + vn * (rad * t.sin())
            })
            .collect();
        Shape::poly(&pts, 0.0, c)
    };
    let soft = accent.mix(WHITE, 0.55);
    let line = GREY.mix(WHITE, 0.25);
    match kind {
        "chart" => {
            let heights = [0.26, 0.40, 0.34, 0.58];
            let mut s: Vec<Shape> = heights
                .iter()
                .enumerate()
                .map(|(i, &ht)| {
                    let c = if i == 3 { accent } else { Rgb::hex(0x9db8f3) };
                    bar(0.13 + i as f64 * 0.20, 0.86 - ht, 0.13, ht, c)
                })
                .collect();
            let trend: Vec<V3> = [(0.10, 0.50), (0.34, 0.36), (0.56, 0.40), (0.88, 0.14)].iter().map(|&(a, b)| p(a, b)).collect();
            s.push(Shape::curve(&trend, 0.016 * h.max(0.3), false, INK));
            s.push(dot(0.88, 0.14, 0.03 * h.max(0.3), INK));
            s
        }
        "ui" => vec![
            dot(0.17, 0.26, 0.11 * h, accent),
            bar(0.33, 0.17, 0.50, 0.075, line),
            bar(0.33, 0.30, 0.32, 0.075, line),
            bar(0.09, 0.52, 0.82, 0.10, soft),
            bar(0.09, 0.70, 0.52, 0.10, accent),
        ],
        "code" => {
            let rows = [
                (0.0, 0.34, INK),
                (0.08, 0.52, accent),
                (0.08, 0.40, Rgb::hex(0x9db8f3)),
                (0.16, 0.30, soft),
                (0.08, 0.46, Rgb::hex(0x9db8f3)),
                (0.0, 0.22, INK),
            ];
            rows.iter().enumerate().map(|(i, &(indent, len, c))| bar(0.09 + indent, 0.13 + i as f64 * 0.13, len, 0.065, c)).collect()
        }
        _ => vec![],
    }
}

/// Folha comprida que afina: cadeia de esferas ao longo de uma haste curva.
fn blade(base: V3, out_dir: V3, len: f64, lean: f64, width: f64, color: Rgb) -> Prim {
    const N: usize = 22;
    let pts: Vec<(V3, f64)> = (0..N)
        .map(|i| {
            let t = i as f64 / (N - 1) as f64;
            let p = base + UP * (len * (t - 0.22 * lean * t * t)) + out_dir * (len * lean * t * t);
            (p, width * (std::f64::consts::PI * (0.08 + 0.92 * t)).sin().max(0.0).powf(0.75) + 0.003)
        })
        .collect();
    let key = [pts[2].0, pts[N / 2].0, pts[N - 2].0];
    Prim::blob(vec![Shape::chain(&pts, color)], &key, 0.0)
}

fn pot(out: &mut Vec<Prim>, cam: &Camera, m: &dyn Fn(V3) -> V3, color: Rgb) {
    cylinder(out, cam, m, v3(0.0, 0.0, 0.0), 0.105, 0.14, 0.22, color);
    cylinder(out, cam, m, v3(0.0, 0.2, 0.0), 0.15, 0.15, 0.04, color.mix(INK, 0.1));
}

fn animal(out: &mut Vec<Prim>, cam: &Camera, xf: &Xf, color: Rgb, dog: bool) {
    let k = xf.k;
    let p = |x: f64, y: f64, z: f64| xf.at(x, y, z);
    let dark = color.mix(INK, 0.16);
    let (len, body_y, head_r) = if dog { (0.24, 0.36, 0.10) } else { (0.17, 0.25, 0.085) };
    let leg_r = if dog { 0.034 } else { 0.026 };
    for (x, z) in [(-0.06, -len), (0.06, -len), (-0.06, len - 0.02), (0.06, len - 0.02)] {
        let far = cam.view(xf.at(x, 0.0, 0.0) - xf.pos).z < 0.0;
        let c = if far { dark } else { color };
        out.push(Prim::blob(
            vec![Shape::hull(&[(p(x, body_y - 0.02, z), leg_r * 1.25 * k), (p(x, leg_r, z + 0.01), leg_r * k)], c)],
            &[p(x, body_y * 0.5, z)],
            if far { -0.02 } else { 0.02 },
        ));
    }
    out.push(Prim::blob(
        vec![Shape::hull(&[(p(0.0, body_y, -len), (body_y * 0.42) * k), (p(0.0, body_y + 0.015, len - 0.02), (body_y * 0.38) * k)], color)],
        &[p(0.0, body_y, -len), p(0.0, body_y, len)],
        0.0,
    ));
    let head = p(0.0, body_y + 0.14, len + 0.11);
    let mut face = vec![Shape::hull(&[(head, head_r * k)], color)];
    if dog {
        face.push(Shape::hull(&[(head, head_r * 0.7 * k), (p(0.0, body_y + 0.11, len + 0.23), 0.045 * k)], color.light()));
        face.push(Shape::hull(&[(p(0.0, body_y + 0.125, len + 0.275), 0.018 * k)], INK));
    }
    for sg in [1.0, -1.0] {
        let side = xf.at(sg, 0.0, 0.0) - xf.pos;
        if cam.view(side).z > -0.3 * k {
            let eye = head + (side.norm() * 0.6 + (xf.at(0.0, 0.0, 1.0) - xf.pos).norm() * 0.75 + UP * 0.2).norm() * (head_r * k);
            face.push(Shape::hull(&[(eye, 0.011 * k)], INK));
        }
    }
    out.push(Prim::blob(face, &[head], 0.03));
    for sg in [1.0, -1.0] {
        let ear = if dog {
            Shape::chain(
                &[
                    (p(sg * 0.07, body_y + 0.20, len + 0.09), 0.035 * k),
                    (p(sg * 0.105, body_y + 0.13, len + 0.08), 0.032 * k),
                    (p(sg * 0.11, body_y + 0.06, len + 0.08), 0.02 * k),
                ],
                dark,
            )
        } else {
            Shape::hull(
                &[(p(sg * 0.048, body_y + 0.20, len + 0.085), 0.03 * k), (p(sg * 0.066, body_y + 0.285, len + 0.07), 0.004 * k)],
                if sg > 0.0 { color } else { dark },
            )
        };
        out.push(Prim::blob(vec![ear], &[p(sg * 0.07, body_y + 0.2, len + 0.09)], if dog { 0.05 } else { 0.0 }));
    }
    let tail: Vec<(V3, f64)> = if dog {
        vec![
            (p(0.0, body_y + 0.08, -len - 0.10), 0.028 * k),
            (p(0.0, body_y + 0.20, -len - 0.17), 0.022 * k),
            (p(0.0, body_y + 0.30, -len - 0.15), 0.012 * k),
        ]
    } else {
        vec![
            (p(0.0, body_y + 0.03, -len - 0.08), 0.024 * k),
            (p(0.0, body_y + 0.14, -len - 0.17), 0.022 * k),
            (p(0.0, body_y + 0.27, -len - 0.15), 0.020 * k),
            (p(0.0, body_y + 0.36, -len - 0.07), 0.016 * k),
        ]
    };
    let key: Vec<V3> = tail.iter().map(|t| t.0).collect();
    out.push(Prim::blob(vec![Shape::chain_smooth(&tail, color)], &key, 0.0));
}

/// Meia-largura e meia-profundidade que cada objeto ocupa no chão.
pub fn footprint(kind: &str) -> Option<(f64, f64)> {
    Some(match kind {
        "desk" => (0.82, 0.44),
        "chair" => (0.30, 0.30),
        "stool" | "plant" | "monstera" => (0.2, 0.2),
        "sofa" => (1.08, 0.5),
        "ball" => (0.26, 0.26),
        "board" => (0.4, 0.24),
        "box" => (0.3, 0.28),
        "lamp" => (0.18, 0.18),
        "dog" => (0.16, 0.36),
        "cat" => (0.13, 0.26),
        _ => return None,
    })
}

pub fn build(kind: &str, xf: Xf, cam: &Camera, style: &Style, color: Option<Rgb>, content_kind: &str, rng: &mut Rng) -> Option<Built> {
    let m = |q: V3| xf.p(q);
    let k = xf.k;
    let mut b = Built::default();
    let accent = color.unwrap_or_else(|| style.accent(rng.below(style.accents.len().min(4))));
    let steel = style.steel();
    b.shadow = footprint(kind);
    let r = 0.012;
    let out = &mut b.prims;
    match kind {
        "desk" => {
            let wood = color.unwrap_or(WOOD);
            for (x, z) in [(-0.68, -0.30), (0.68, -0.30), (-0.68, 0.30), (0.68, 0.30)] {
                push_box(out, cam, &m, v3(x, 0.355, z), v3(0.055, 0.71, 0.055), steel, 0.006);
            }
            push_box(out, cam, &m, v3(0.42, 0.60, 0.0), v3(0.44, 0.20, 0.62), PAPER.mix(GREY, 0.35), 0.006);
            push_box(out, cam, &m, v3(0.0, 0.735, 0.0), v3(1.5, 0.05, 0.76), wood, 0.008);
            b.anchors.push(("touch".into(), xf.at(0.0, 0.775, 0.22)));
            b.top = 0.76;
        }
        "chair" => {
            for (x, z) in [(-0.19, -0.19), (0.19, -0.19), (-0.19, 0.19), (0.19, 0.19)] {
                push_box(out, cam, &m, v3(x, 0.195, z), v3(0.04, 0.39, 0.04), steel, 0.005);
            }
            push_box(out, cam, &m, v3(0.0, 0.415, 0.0), v3(0.46, 0.05, 0.46), accent, r);
            push_box(out, cam, &m, v3(0.0, 0.69, -0.21), v3(0.46, 0.50, 0.045), accent, r);
            b.anchors.push(("seat".into(), xf.at(0.0, 0.44, -0.02)));
            b.top = 0.44;
        }
        "stool" => {
            cylinder(out, cam, &m, v3(0.0, 0.0, 0.0), 0.16, 0.15, 0.03, steel);
            cylinder(out, cam, &m, v3(0.0, 0.03, 0.0), 0.022, 0.022, 0.59, steel);
            cylinder(out, cam, &m, v3(0.0, 0.62, 0.0), 0.17, 0.18, 0.05, accent);
            b.anchors.push(("seat".into(), xf.at(0.0, 0.67, 0.0)));
            b.top = 0.67;
        }
        "sofa" => {
            for (x, z) in [(-0.85, -0.3), (0.85, -0.3), (-0.85, 0.32), (0.85, 0.32)] {
                push_box(out, cam, &m, v3(x, 0.05, z), v3(0.06, 0.10, 0.06), steel, 0.004);
            }
            push_box(out, cam, &m, v3(0.0, 0.24, 0.02), v3(1.72, 0.26, 0.80), accent, r);
            push_box(out, cam, &m, v3(0.0, 0.50, -0.31), v3(1.72, 0.66, 0.20), accent, r);
            for x in [-0.42, 0.42] {
                push_box(out, cam, &m, v3(x, 0.39, 0.07), v3(0.82, 0.08, 0.64), accent.mix(WHITE, 0.3), r);
            }
            for x in [-0.95, 0.95] {
                push_box(out, cam, &m, v3(x, 0.38, 0.02), v3(0.20, 0.54, 0.84), accent.mix(INK, 0.06), r);
            }
            b.anchors.push(("seat0".into(), xf.at(-0.42, 0.43, 0.06)));
            b.anchors.push(("seat1".into(), xf.at(0.42, 0.43, 0.06)));
            b.top = 0.43;
        }
        "ball" => {
            let c = xf.at(0.0, 0.30, 0.0);
            let shine = c + (cam.up() * 0.14 - cam.right() * 0.12 + cam.toward() * 0.2) * k;
            out.push(Prim::blob(
                vec![Shape::hull(&[(c, 0.30 * k)], accent).shaded(style.shading), Shape::hull(&[(shine, 0.05 * k)], accent.light())],
                &[c],
                0.0,
            ));
            b.anchors.push(("seat".into(), xf.at(0.0, 0.57, 0.0)));
            b.top = 0.57;
        }
        "laptop" => {
            push_box(out, cam, &m, v3(0.0, 0.008, 0.0), v3(0.36, 0.016, 0.25), GREY, 0.004);
            let tilt = 0.30_f64;
            let lid = |q: V3| xf.p(v3(0.0, 0.016, -0.12) + q.rot_x(-tilt));
            let mut faces = boxed(cam, &lid, v3(0.0, 0.12, 0.0), v3(0.36, 0.24, 0.012), SLATE.mix(INK, 0.2), 0.004);
            if let Some(front) = faces[4].as_mut() {
                let (o, right, down) = (lid(v3(-0.165, 0.225, 0.0065)), lid(v3(0.165, 0.225, 0.0065)), lid(v3(-0.165, 0.018, 0.0065)));
                front.shapes.push(Shape::poly(&[o, right, right + (down - o), down], 0.0, PAPER));
                front.shapes.extend(content(content_kind, o, right - o, down - o, accent));
            }
            if let Some(back) = faces[5].as_mut() {
                let logo: Vec<V3> = (0..16)
                    .map(|i| {
                        let t = TAU * i as f64 / 16.0;
                        lid(v3(0.022 * t.cos(), 0.12 + 0.022 * t.sin(), -0.0065))
                    })
                    .collect();
                back.shapes.push(Shape::poly(&logo, 0.0, WHITE));
            }
            out.extend(faces.into_iter().flatten());
            b.anchors.push(("use".into(), xf.at(0.0, 0.035, 0.05)));
            b.top = 0.016;
        }
        "mug" => {
            let handle: Vec<V3> =
                [(0.04, 0.078), (0.082, 0.072), (0.086, 0.036), (0.04, 0.022)].iter().map(|&(x, y)| xf.at(x, y, 0.0)).collect();
            out.push(Prim::blob(vec![Shape::curve(&handle, 0.014 * k, true, accent.shade())], &handle[1..3], 0.0));
            cylinder(out, cam, &m, v3(0.0, 0.0, 0.0), 0.04, 0.045, 0.098, accent);
            b.anchors.push(("touch".into(), xf.at(0.0, 0.06, 0.0)));
            b.top = 0.098;
        }
        "books" => {
            let stack = [(0.0, 0.26, 0.19, 0.045, 0), (0.14, 0.23, 0.17, 0.04, 1), (-0.10, 0.20, 0.15, 0.035, 3)];
            let mut y = 0.0;
            for (yaw, w, d, h, slot) in stack {
                let mm = |q: V3| xf.p(q.rot_y(yaw));
                push_box(out, cam, &mm, v3(0.0, y + h / 2.0, 0.0), v3(w, h, d), style.accent(slot), 0.004);
                y += h;
            }
            b.anchors.push(("touch".into(), xf.at(0.0, y + 0.01, 0.0)));
            b.top = y;
        }
        "plant" => {
            pot(out, cam, &m, color.unwrap_or(style.accent(0)));
            let greens = [LEAF, LEAF_DARK, LEAF_LIGHT];
            for i in 0..9 {
                let a = i as f64 * 2.399 + rng.range(-0.2, 0.2);
                let dir = v3(a.cos(), 0.0, a.sin());
                let len = rng.range(0.55, 0.95) * k;
                let lean = rng.range(0.18, 0.62);
                out.push(blade(xf.at(dir.x * 0.05, 0.22, dir.z * 0.05), dir.rot_y(xf.yaw), len, lean, 0.055 * k, greens[i % 3]));
            }
            b.top = 0.24;
        }
        "monstera" => {
            pot(out, cam, &m, color.unwrap_or(PAPER.mix(GREY, 0.5)));
            let greens = [LEAF, LEAF_DARK, LEAF_LIGHT];
            for i in 0..6 {
                let a = i as f64 * 2.399 + rng.range(-0.25, 0.25);
                let dir = v3(a.cos(), 0.0, a.sin()).rot_y(xf.yaw);
                let reach = rng.range(0.16, 0.42) * k;
                let height = (0.55 + 0.16 * (i % 3) as f64 + rng.range(-0.05, 0.05)) * k;
                let base = xf.at(0.0, 0.22, 0.0);
                let tip = base + UP * height + dir * reach;
                let mid = base + UP * (height * 0.55) + dir * (reach * 0.25);
                out.push(Prim::blob(vec![Shape::curve(&[base, mid, tip], 0.014 * k, true, LEAF_DARK)], &[mid], 0.02));
                let grow = (dir * 0.55 + UP * 0.83).norm();
                let c = greens[i % 3];
                let leaf = vec![
                    Shape::hull(&[(tip, 0.03 * k), (tip + grow * (0.13 * k), 0.125 * k), (tip + grow * (0.31 * k), 0.022 * k)], c),
                    Shape::curve(&[tip, tip + grow * (0.27 * k)], 0.008 * k, false, c.mix(WHITE, 0.35)),
                ];
                out.push(Prim::blob(leaf, &[tip + grow * (0.13 * k)], 0.0));
            }
            b.top = 0.24;
        }
        "board" => {
            cylinder(out, cam, &m, v3(0.0, 0.0, -0.03), 0.24, 0.22, 0.03, steel);
            push_box(out, cam, &m, v3(0.0, 0.52, -0.03), v3(0.05, 1.0, 0.05), steel, 0.004);
            let mut faces = boxed(cam, &m, v3(0.0, 1.34, 0.0), v3(1.34, 0.88, 0.04), PAPER, 0.02);
            if let Some(front) = faces[4].as_mut() {
                let (o, right, down) = (xf.at(-0.67, 1.78, 0.021), xf.at(0.67, 1.78, 0.021), xf.at(-0.67, 0.90, 0.021));
                front.shapes.extend(content(content_kind, o, right - o, down - o, accent));
            }
            if let Some(back) = faces[5].as_mut() {
                back.shapes[0].fill = GREY;
            }
            out.extend(faces.into_iter().flatten());
            b.anchors.push(("touch".into(), xf.at(0.36, 1.42, 0.03)));
            b.top = 0.0;
        }
        "card" => {
            let (w, h) = (0.66 * k, 0.42 * k);
            let roll = rng.range(-0.10, 0.10);
            let right = (cam.right() * roll.cos() + cam.up() * roll.sin()) * w;
            let down = (cam.right() * roll.sin() - cam.up() * roll.cos()) * h;
            let o = xf.pos - right * 0.5 - down * 0.5;
            let rad = 0.035 * k;
            let rect = |o: V3| {
                let (ri, di) = (right.norm() * rad, down.norm() * rad);
                [o + ri + di, o + right - ri + di, o + right - ri + down - di, o + ri + down - di]
            };
            let drop = (cam.right() * 0.02 - cam.up() * 0.03) * k;
            let mut shapes = vec![Shape::poly(&rect(o + drop), rad, INK).alpha(0.12), Shape::poly(&rect(o), rad, PAPER)];
            shapes.extend(content(content_kind, o, right, down, accent));
            out.push(Prim::face(shapes, &rect(o), cam.toward()));
        }
        "box" => {
            let c = color.unwrap_or(style.accent(3));
            push_box(out, cam, &m, v3(0.0, 0.19, 0.0), v3(0.44, 0.38, 0.40), c, 0.008);
            b.anchors.push(("touch".into(), xf.at(0.0, 0.39, 0.0)));
            b.top = 0.38;
        }
        "lamp" => {
            cylinder(out, cam, &m, v3(0.0, 0.0, 0.0), 0.15, 0.14, 0.03, steel);
            cylinder(out, cam, &m, v3(0.0, 0.03, 0.0), 0.014, 0.014, 1.40, steel);
            cylinder(out, cam, &m, v3(0.0, 1.40, 0.0), 0.21, 0.12, 0.26, color.unwrap_or(style.accent(3)));
            b.top = 0.0;
        }
        "cat" | "dog" => {
            let dog = kind == "dog";
            let c = color.unwrap_or(if dog { Rgb::hex(0xffc043) } else { Rgb::hex(0xff9a4a) });
            animal(out, cam, &xf, c, dog);
        }
        _ => return None,
    }
    Some(b)
}

/// Objeto na mão do personagem. `xf` já está na mão, virado como o personagem.
pub fn held(kind: &str, xf: Xf, cam: &Camera, accent: Rgb) -> Vec<Prim> {
    let m = |q: V3| xf.p(q);
    let mut out = Vec::new();
    match kind {
        "phone" | "tablet" => {
            let (w, h) = if kind == "phone" { (0.08, 0.155) } else { (0.20, 0.27) };
            // A tela fica virada para quem segura.
            let mut faces = boxed(cam, &m, v3(0.0, h / 2.0 - 0.03, 0.0), v3(w, h, 0.012), INK, 0.006);
            if let Some(screen) = faces[5].as_mut() {
                let (o, right, down) = (
                    xf.at(w / 2.0 - 0.008, h - 0.04, -0.0065),
                    xf.at(-w / 2.0 + 0.008, h - 0.04, -0.0065),
                    xf.at(w / 2.0 - 0.008, -0.02, -0.0065),
                );
                screen.shapes.push(Shape::poly(&[o, right, right + (down - o), down], 0.0, PAPER));
                screen.shapes.extend(content(if kind == "phone" { "blank" } else { "ui" }, o, right - o, down - o, accent));
            }
            out.extend(faces.into_iter().flatten());
        }
        "mug" => cylinder(&mut out, cam, &m, v3(0.0, -0.05, 0.0), 0.04, 0.045, 0.098, accent),
        "book" => push_box(&mut out, cam, &m, v3(0.0, 0.07, 0.0), v3(0.17, 0.23, 0.035), accent, 0.005),
        "box" => push_box(&mut out, cam, &m, v3(0.0, 0.0, 0.0), v3(0.40, 0.30, 0.30), accent, 0.008),
        _ => {}
    }
    out
}

/// Objeto próprio, descrito no JSON com formas simples em metros.
pub fn build_custom(def: &CustomDef, xf: Xf, cam: &Camera, style: &Style, color: Option<Rgb>, rng: &mut Rng) -> Built {
    let mut b = Built::default();
    let first = rng.below(style.accents.len());
    let accent = color.unwrap_or_else(|| style.accent(first));
    let paint = |name: &Option<String>| -> Rgb {
        match name.as_deref() {
            None | Some("accent") => accent,
            Some("accent2") => style.accent(first + 1),
            Some("steel") => style.steel(),
            Some(other) => color::resolve(other).unwrap_or(accent),
        }
    };
    let at = |a: &[f64; 3]| v3(a[0], a[1], a[2]);
    // Extensão no chão e altura, para a sombra e o apoio quando o JSON não disser.
    let (mut rx, mut rz, mut top) = (0.05_f64, 0.05_f64, 0.0_f64);
    let mut reach = |c: V3, hx: f64, hy: f64, hz: f64| {
        rx = rx.max(c.x.abs() + hx);
        rz = rz.max(c.z.abs() + hz);
        top = top.max(c.y + hy);
    };
    let k = xf.k;
    for part in &def.parts {
        match part {
            Part::Box { at: c, size, color, round, yaw } => {
                let (c, yaw) = (at(c), yaw.unwrap_or(0.0).to_radians());
                let m = |q: V3| xf.p(c + q.rot_y(yaw));
                let size = v3(size[0], size[1], size[2]);
                let spread = size.x.max(size.z) / 2.0;
                reach(c, spread, size.y / 2.0, spread);
                push_box(&mut b.prims, cam, &m, V3::default(), size, paint(color), round.unwrap_or(0.008));
            }
            Part::Cylinder { at: c, r, r_top, h, color } => {
                let c = at(c);
                let m = |q: V3| xf.p(q);
                reach(c, *r, *h, *r);
                cylinder(&mut b.prims, cam, &m, c, *r, r_top.unwrap_or(*r), *h, paint(color));
            }
            Part::Sphere { at: c, r, color } => {
                let c = at(c);
                reach(c, *r, *r, *r);
                b.prims.push(Prim::blob(vec![Shape::hull(&[(xf.p(c), r * k)], paint(color)).shaded(style.shading)], &[xf.p(c)], 0.0));
            }
            Part::Hull { points, color } | Part::Chain { points, color } => {
                let discs: Vec<(V3, f64)> = points.iter().map(|q| (xf.at(q[0], q[1], q[2]), q[3] * k)).collect();
                for q in points {
                    reach(v3(q[0], q[1], q[2]), q[3], q[3], q[3]);
                }
                let key: Vec<V3> = discs.iter().map(|d| d.0).collect();
                let shape = match part {
                    Part::Chain { .. } => Shape::chain_smooth(&discs, paint(color)),
                    _ => Shape::hull(&discs, paint(color)).shaded(style.shading),
                };
                b.prims.push(Prim::blob(vec![shape], &key, 0.0));
            }
            Part::Panel { at: c, size, content: kind, color, yaw } => {
                let (c, yaw) = (at(c), yaw.unwrap_or(0.0).to_radians());
                let m = |q: V3| xf.p(c + q.rot_y(yaw));
                let (w, h) = (size[0], size[1]);
                reach(c, w / 2.0, h / 2.0, w / 2.0);
                let mut faces = boxed(cam, &m, V3::default(), v3(w, h, 0.03), PAPER, 0.012);
                if let Some(front) = faces[4].as_mut() {
                    let (o, right, down) =
                        (m(v3(-w / 2.0, h / 2.0, 0.016)), m(v3(w / 2.0, h / 2.0, 0.016)), m(v3(-w / 2.0, -h / 2.0, 0.016)));
                    front.shapes.extend(content(kind.as_deref().unwrap_or("ui"), o, right - o, down - o, paint(color)));
                }
                if let Some(back) = faces[5].as_mut() {
                    back.shapes[0].fill = GREY;
                }
                b.prims.extend(faces.into_iter().flatten());
            }
        }
    }
    for (name, point) in [("seat", &def.seat), ("use", &def.use_point), ("touch", &def.touch)] {
        if let Some(p) = point {
            b.anchors.push((name.into(), xf.at(p[0], p[1], p[2])));
        }
    }
    b.top = def.top.unwrap_or(top);
    b.shadow = Some(def.shadow.map(|s| (s[0], s[1])).unwrap_or((rx.min(1.5), rz.min(1.5))));
    b
}

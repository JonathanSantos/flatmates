//! Cabeça, cabelo, barba e acessórios de cabeça. O cabelo é uma calota da
//! esfera da cabeça, então a linha do cabelo sai certa de frente, de perfil
//! e de costas sem desenho especial para cada vista.
use super::draw::Look;
use super::{Rig, SIGN};
use crate::camera::Camera;
use crate::color::{Rgb, INK, WHITE};
use crate::math::Det;
use crate::math::V3;
use crate::prim::{Prim, Shape};

pub fn draw(rig: &Rig, look: &Look, cam: &Camera) -> Vec<Prim> {
    let d = &rig.d;
    let s = d.s;
    let r = d.head_r;
    let c = rig.head;
    let (f, u) = (rig.head_fwd, rig.head_up);
    let side = u.cross(f).norm();
    let hair = look.hair_color;
    let skin = look.skin;
    let seen = |dir: V3| cam.view(dir).z;
    let at = |fwd: f64, up: f64, sd: f64| c + (f * fwd + u * up + side * sd) * r;
    let crown = (u * 0.84 - f * 0.54).norm();
    let mut out: Vec<Prim> = Vec::new();
    let mut shapes: Vec<Shape> = Vec::new();

    let neck = Shape::hull(&[(rig.chest + rig.spine * (0.02 * s), 0.044 * s), (c, 0.044 * s)], skin.mix(INK, 0.09));

    if look.hair == "afro" {
        // Volume grande atrás, e só a calota do rosto em cor de pele.
        shapes.push(Shape::hull(&[(at(-0.12, 0.22, 0.0), r * 1.42)], hair));
        shapes.push(neck);
        shapes.push(Shape::cap(c, r, f * 0.92 - u * 0.38, 68.0, skin));
        if seen(f) > -0.15 {
            shapes.push(Shape::hull(&[(at(0.40, -0.26, 0.0), r * 0.62), (at(0.20, -0.40, 0.0), r * 0.76)], skin));
        }
    } else {
        shapes.push(neck);
        // Esfera do crânio mais o queixo: a cabeça vira um oval que aponta
        // para a frente, em vez de uma bola sem direção.
        // Mandíbula larga (dá o oval de frente) e queixo adiantado (deixa o
        // rosto de pé no perfil, em vez de apontar para cima).
        shapes.push(Shape::hull(
            &[(c, r), (at(0.16, -0.36, 0.0), r * 0.78), (at(0.50, -0.62, 0.0), r * 0.44), (at(1.0, -0.12, 0.0), 0.027 * s)],
            skin,
        ));
    }

    // Orelhas: só as que não estão escondidas atrás da cabeça.
    for sg in SIGN {
        if seen(side * sg) > -0.25 && look.hair != "afro" {
            shapes.push(Shape::hull(&[(at(-0.05, -0.06, sg * 0.97), 0.024 * s)], skin.mix(INK, 0.07)));
        }
    }

    match look.hair.as_str() {
        "short" | "bun" | "ponytail" => {
            // Topo com a linha do cabelo quase horizontal, acima da orelha,
            // e a nuca coberta. De perfil o rosto fica de frente, não para cima.
            shapes.push(Shape::cap(c, r * 1.07, u * 0.95 - f * 0.31, 64.0, hair));
            shapes.push(Shape::cap(c, r * 1.07, u * 0.30 - f * 0.95, 66.0, hair));
            // Franja curta: traz a linha do cabelo para a testa na vista de frente.
            shapes.push(Shape::cap(c, r * 1.07, u * 0.90 + f * 0.43, 40.0, hair));
        }
        "curly" => {
            shapes.push(Shape::cap(c, r * 1.06, u * 0.95 - f * 0.31, 62.0, hair));
            shapes.push(Shape::cap(c, r * 1.06, u * 0.30 - f * 0.95, 66.0, hair));
            let tufts = [
                (0.30, 0.95, 0.0),
                (0.10, 1.0, 0.45),
                (0.10, 1.0, -0.45),
                (-0.35, 0.95, 0.0),
                (-0.25, 0.60, 0.80),
                (-0.25, 0.60, -0.80),
                (-0.85, 0.45, 0.35),
                (-0.85, 0.45, -0.35),
                (0.22, 0.74, 0.66),
                (0.22, 0.74, -0.66),
                (-0.95, -0.05, 0.0),
            ];
            for (a, b, e) in tufts {
                let dir = (f * a + u * b + side * e).norm();
                if seen(dir) > -0.35 {
                    shapes.push(Shape::hull(&[(c + dir * (r * 1.02), r * 0.36)], hair));
                }
            }
        }
        "bob" | "long" => {
            // Calota larga atrás e nos lados, mais uma franja na testa.
            shapes.push(Shape::cap(c, r * 1.08, u * 0.44 - f * 0.90, 81.0, hair));
            shapes.push(Shape::cap(c, r * 1.08, u * 0.90 + f * 0.44, 42.0, hair));
        }
        _ => {}
    }

    if look.beard {
        // Faixa que contorna a mandíbula, por baixo da linha da boca.
        shapes.push(Shape::cap(c, r * 1.03, f * 0.25 - u * 0.97, 36.0, hair));
        if seen(f) > -0.15 {
            let jaw = [(at(0.18, -0.66, 0.50), r * 0.34), (at(0.18, -0.66, -0.50), r * 0.34), (at(0.50, -0.70, 0.0), r * 0.42)];
            shapes.push(Shape::hull(&jaw, hair));
        }
    }

    // Brilho no cabelo, do lado da luz (em cima, à esquerda da câmera).
    let lit = (cam.up() * 0.72 - cam.right() * 0.52 + cam.toward() * 0.46).norm();
    match look.hair.as_str() {
        "bald" => {}
        "afro" => {
            let center = at(-0.12, 0.22, 0.0);
            shapes.push(Shape::hull(&[(center + lit * (r * 0.95), r * 0.20)], hair.mix(WHITE, 0.13)));
        }
        _ if lit.dot(crown) > 0.45 => shapes.push(Shape::cap(c, r * 1.085, lit, 17.0, hair.mix(WHITE, 0.14))),
        _ => {}
    }

    if look.accessories.iter().any(|a| a == "beanie") {
        let color = look.top_color.mix(INK, 0.25);
        shapes.push(Shape::cap(c, r * 1.13, u * 0.94 - f * 0.34, 70.0, color));
        shapes.push(Shape::hull(&[(c + (u * 0.94 - f * 0.34).norm() * (r * 1.2), r * 0.24)], color.light()));
    }

    if look.accessories.iter().any(|a| a == "headphones") {
        let band: Vec<V3> = (0..=8)
            .map(|k| {
                let t = std::f64::consts::PI * k as f64 / 8.0;
                c + (side * t.dcos() + u * t.dsin()) * (r * 1.15)
            })
            .collect();
        shapes.push(Shape::curve(&band, 0.022 * s, true, INK));
        for sg in SIGN {
            if seen(side * sg) > -0.2 {
                shapes.push(Shape::hull(&[(at(0.0, -0.04, sg * 1.08), 0.042 * s)], Rgb::hex(0xff6b5b)));
            }
        }
    }

    if seen(f) > -0.2 && look.face != "none" {
        let on_face = |yaw: f64, pitch: f64| {
            let dir = f * (yaw.dcos() * pitch.dcos()) + side * (yaw.dsin() * pitch.dcos()) + u * pitch.dsin();
            c + dir * (r * 1.005)
        };
        // Cada olho só aparece se a parte do rosto onde ele fica estiver à vista.
        for sg in SIGN {
            let eye = on_face(sg * 0.40, 0.02);
            if seen(eye - c) > 0.12 * r {
                shapes.push(Shape::hull(&[(eye, 0.011 * s)], INK));
            }
        }
        if look.accessories.iter().any(|a| a == "glasses") {
            // Aro em volta de cada olho à vista, e a ponte quando os dois aparecem.
            let mut shown = 0;
            for sg in SIGN {
                let eye = on_face(sg * 0.40, 0.02);
                if seen(eye - c) <= 0.12 * r {
                    continue;
                }
                shown += 1;
                let ring: Vec<V3> = (0..=16)
                    .map(|k| {
                        let t = std::f64::consts::TAU * k as f64 / 16.0;
                        on_face(sg * 0.40 + 0.22 * t.dcos(), 0.02 + 0.21 * t.dsin())
                    })
                    .collect();
                shapes.push(Shape::poly(&ring, 0.0, crate::color::WHITE).alpha(0.32));
                shapes.push(Shape::curve(&ring, 0.012 * s, true, INK));
            }
            if shown == 2 {
                shapes.push(Shape::curve(&[on_face(-0.18, 0.04), on_face(0.0, 0.07), on_face(0.18, 0.04)], 0.012 * s, true, INK));
            }
        }
        if look.face == "smile" && !look.beard {
            for sg in SIGN {
                let cheek = on_face(sg * 0.66, -0.26);
                if seen(cheek - c) > 0.30 * r {
                    shapes.push(Shape::hull(&[(cheek, 0.019 * s)], skin.mix(Rgb::hex(0xff5c5c), 0.30)));
                }
            }
        }
        if look.face == "smile" && seen(f) > 0.6 {
            let arc: Vec<V3> = (-3..=3).map(|k| on_face(k as f64 * 0.11, -0.44 + 0.006 * (k * k) as f64 * 3.0)).collect();
            shapes.push(Shape::curve(&arc, 0.011 * s, true, INK));
        }
    }

    out.push(Prim::blob(shapes, &[c], 0.14));

    // Volumes de cabelo que ficam fora da cabeça têm profundidade própria:
    // atrás do tronco quando o personagem olha para a câmera, na frente
    // quando está de costas.
    match look.hair.as_str() {
        "bun" => {
            let p = c + crown * (r * 1.28);
            out.push(Prim::blob(vec![Shape::hull(&[(p, r * 0.46)], hair)], &[p], 0.14));
        }
        "ponytail" => {
            let pts = [
                (at(-0.95, 0.30, 0.0), 0.036 * s),
                (at(-1.75, 0.10, 0.0), 0.046 * s),
                (at(-2.05, -0.90, 0.0), 0.034 * s),
                (at(-1.85, -1.70, 0.0), 0.010 * s),
            ];
            let key: Vec<V3> = pts.iter().map(|p| p.0).collect();
            out.push(Prim::blob(vec![Shape::chain_smooth(&pts, hair)], &key[1..3], 0.12));
        }
        "bob" => {
            // Volume que desce atrás da nuca até a linha do queixo.
            let pts = [(at(-0.36, -0.15, 0.0), r * 0.98), (at(-0.44, -0.92, 0.0), r * 0.74)];
            out.push(Prim::blob(vec![Shape::hull(&pts, hair)], &[pts[0].0, pts[1].0], 0.12));
        }
        "long" => {
            let back = rig.chest - rig.fwd * (0.075 * s);
            let pts =
                [(at(-0.30, -0.10, 0.0), r * 1.05), (back + rig.spine * (0.02 * s), 0.088 * s), (back - rig.spine * (0.13 * s), 0.07 * s)];
            out.push(Prim::blob(vec![Shape::chain_smooth(&pts, hair)], &[pts[1].0, pts[2].0], 0.10));
        }
        _ => {}
    }
    out
}

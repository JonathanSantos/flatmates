//! Aparência do corpo e das roupas, montada em cima das juntas do rig.
use super::head;
use super::pose::Grip;
use super::{Rig, SIGN, SOLE_BACK, SOLE_FRONT, SOLE_R};
use crate::camera::Camera;
use crate::color::{Rgb, INK, WHITE};
use crate::math::V3;
use crate::prim::{Prim, Shape};

pub const OUTFITS: [&str; 7] = ["tshirt", "shirt", "hoodie", "sweater", "blazer", "tank", "dress"];
pub const BOTTOMS: [&str; 3] = ["pants", "shorts", "skirt"];
pub const FACES: [&str; 3] = ["none", "dots", "smile"];
pub const HAIRS: [&str; 8] = ["short", "bob", "long", "bun", "ponytail", "afro", "curly", "bald"];
pub const ACCESSORIES: [&str; 3] = ["beanie", "headphones", "backpack"];

/// Corte da parte de cima: quanto cobre e quanta folga tem.
#[derive(Clone, Copy, Debug)]
pub struct Outfit {
    /// Fração do braço coberta pela manga: 0 sem manga, 1 manga longa.
    pub sleeve: f64,
    /// Folga em metros somada aos raios do corpo.
    pub loose: f64,
    /// Caimento: 0 acompanha o corpo (marca a cintura), 1 cai reto a partir
    /// do ponto mais largo.
    pub drape: f64,
    /// Quanto a barra desce além do quadril.
    pub hem: f64,
    pub hood: bool,
    pub lapel: bool,
    pub pocket: bool,
    pub collar: bool,
    /// Malha canelada: punhos e barra em faixa mais escura.
    pub ribbed: bool,
    /// Vestido: fração da coxa coberta pela saia.
    pub skirt: Option<f64>,
}

pub fn outfit(name: &str) -> Option<Outfit> {
    let base = Outfit {
        sleeve: 1.0,
        loose: 0.010,
        drape: 0.5,
        hem: 0.0,
        hood: false,
        lapel: false,
        pocket: false,
        collar: false,
        ribbed: false,
        skirt: None,
    };
    Some(match name {
        "tshirt" => Outfit { sleeve: 0.52, drape: 0.42, ..base },
        "shirt" => Outfit { collar: true, ..base },
        "hoodie" => Outfit { loose: 0.024, drape: 1.0, hem: 0.02, hood: true, pocket: true, ribbed: true, ..base },
        "sweater" => Outfit { loose: 0.018, drape: 0.8, ribbed: true, ..base },
        "blazer" => Outfit { loose: 0.016, drape: 0.72, hem: 0.07, lapel: true, ..base },
        "tank" => Outfit { sleeve: 0.0, loose: 0.004, drape: 0.0, ..base },
        "dress" => Outfit { sleeve: 0.34, loose: 0.006, drape: 0.1, skirt: Some(0.92), ..base },
        _ => return None,
    })
}

#[derive(Clone, Debug)]
pub struct Look {
    pub skin: Rgb,
    pub hair: String,
    pub hair_color: Rgb,
    pub outfit: Outfit,
    pub top_color: Rgb,
    pub bottom: String,
    pub bottom_color: Rgb,
    pub shoe_color: Rgb,
    pub face: String,
    pub beard: bool,
    pub accessories: Vec<String>,
}

/// Tom do membro do lado de lá.
fn far_tone(c: Rgb) -> Rgb {
    c.mix(INK, 0.14)
}

/// Ponta reta de um tubo (barra de calça, punho): duas esferas pequenas nos
/// cantos da silhueta, em vez de uma ponta redonda.
fn flat_end(c: V3, r: f64, axis: V3, cam: &Camera) -> Vec<(V3, f64)> {
    let perp = axis.cross(cam.toward());
    if perp.len() < 0.35 {
        return vec![(c, r)];
    }
    let e = (r * 0.2).min(0.008);
    let off = perp.norm() * (r - e);
    vec![(c + off, e), (c - off, e)]
}

/// Tubo que começa redondo em `a` e termina reto em `b`.
fn cuffed(a: V3, ra: f64, b: V3, rb: f64, cam: &Camera, fill: Rgb) -> Shape {
    let mut discs = vec![(a, ra)];
    discs.extend(flat_end(b, rb, (b - a).norm(), cam));
    Shape::hull(&discs, fill)
}

/// Um osso do membro, do ponto `a` ao `b`, com pele e pano.
struct Bone {
    a: V3,
    ra: f64,
    b: V3,
    rb: f64,
    skin: Rgb,
    cloth: Rgb,
    /// Fração coberta pelo pano a partir de `a` (0 nada, 1 tudo).
    cover: f64,
    /// Folga do pano sobre a pele.
    loose: f64,
    /// Quanto a barra abre além da folga.
    flare: f64,
    /// Punho canelado: uma faixa mais escura onde o pano termina.
    band: Option<Rgb>,
    bias: f64,
}

/// Desenha o osso em duas metades com profundidades próprias: assim o
/// antebraço pode passar por cima de uma mesa com o cotovelo embaixo dela.
fn bone(out: &mut Vec<Prim>, cam: &Camera, bn: &Bone) {
    let at = |t: f64| bn.a.lerp(bn.b, t);
    let r = |t: f64| bn.ra + (bn.rb - bn.ra) * t;
    for (t0, t1) in [(0.0, 0.5), (0.5, 1.0)] {
        let mut shapes = Vec::new();
        if bn.cover < t1 {
            shapes.push(Shape::hull(&[(at(t0), r(t0)), (at(t1), r(t1))], bn.skin));
        }
        if bn.cover > t0 {
            let end = bn.cover.min(t1);
            if bn.cover >= 1.0 || bn.cover > t1 {
                shapes.push(Shape::hull(&[(at(t0), r(t0) + bn.loose), (at(end), r(end) + bn.loose)], bn.cloth));
            } else {
                shapes.push(cuffed(at(t0), r(t0) + bn.loose, at(end), r(end) + bn.loose + bn.flare, cam, bn.cloth));
                if let Some(color) = bn.band {
                    let from = (end - 0.16).max(t0);
                    let grip = bn.loose * 0.55;
                    shapes.push(cuffed(at(from), r(from) + grip, at(end), r(end) + grip + 0.003, cam, color));
                }
            }
        }
        out.push(Prim::blob(shapes, &[at(t0), at(t1)], bn.bias));
    }
}

/// Mão desenhada conforme o papel dela na pose.
fn hand_shapes(rig: &Rig, i: usize, skin: Rgb) -> Vec<Shape> {
    let d = &rig.d;
    let s = d.s;
    let (el, wr) = (rig.elbow[i], rig.wrist[i]);
    let dir = (wr - el).norm();
    let raw = rig.fwd * 0.7 + rig.spine * 0.7;
    let thumb_dir = (raw - dir * raw.dot(dir)).norm();
    let palm = wr + dir * (0.045 * s);
    let finger =
        |from: V3, toward: V3, len: f64, r: f64| Shape::hull(&[(from, r * s), (from + toward.norm() * (len * s), r * 0.86 * s)], skin);
    match rig.grips[i] {
        Grip::Relaxed => vec![
            Shape::hull(&[(wr, d.r_wrist), (palm, 0.052 * s), (wr + dir * (0.118 * s), 0.040 * s)], skin),
            finger(palm, dir * 0.45 + thumb_dir, 0.068, 0.020),
        ],
        Grip::Fist => vec![
            Shape::hull(&[(wr, d.r_wrist), (wr + dir * (0.055 * s), 0.054 * s)], skin),
            finger(palm, dir * 0.9 + thumb_dir * 0.6, 0.05, 0.02),
        ],
        Grip::Point => vec![
            Shape::hull(&[(wr, d.r_wrist), (wr + dir * (0.05 * s), 0.05 * s)], skin),
            finger(wr + dir * (0.07 * s), dir, 0.095, 0.017),
            finger(palm, dir * 0.3 + thumb_dir, 0.058, 0.018),
        ],
        Grip::Open => {
            // Palma aberta: uma pá larga no plano da mão e o polegar para dentro.
            // Vista de lado ela continua lendo como mão, sem virar um dedo solto.
            let across = dir.cross(rig.fwd);
            let across = if across.len() > 0.3 { across.norm() } else { rig.side };
            let inward = if across.dot(rig.side) * SIGN[i] > 0.0 { -across } else { across };
            let tip = wr + dir * (0.122 * s);
            vec![
                Shape::hull(
                    &[
                        (wr, d.r_wrist),
                        (palm + across * (0.022 * s), 0.036 * s),
                        (palm - across * (0.022 * s), 0.036 * s),
                        (tip + across * (0.026 * s), 0.026 * s),
                        (tip - across * (0.026 * s), 0.026 * s),
                    ],
                    skin,
                ),
                finger(palm, dir * 0.35 + inward, 0.078, 0.019),
            ]
        }
    }
}

pub fn draw(rig: &Rig, look: &Look, cam: &Camera, shading: bool) -> Vec<Prim> {
    let d = &rig.d;
    let s = d.s;
    let o = &look.outfit;
    let depth = |p: V3| cam.view(p).z;
    let is_far = |pair: &[V3; 2], i: usize| depth(pair[1 - i]) - depth(pair[i]) > 0.04 * s;
    let tone = |c: Rgb, far: bool| if far { far_tone(c) } else { c };
    let mut out: Vec<Prim> = Vec::new();

    let dress = o.skirt.is_some();
    let bottom = if dress { "skirt" } else { look.bottom.as_str() };
    let leg_loose = 0.007 * s;

    for i in 0..2 {
        let far = is_far(&rig.hip, i);
        let skin = tone(look.skin, far);
        let cloth = tone(look.bottom_color, far);
        let (hip, knee, ankle) = (rig.hip[i], rig.knee[i], rig.ankle[i]);

        let (thigh_cover, shin_cover, flare) = match bottom {
            "pants" => (1.0, 0.86, 0.011 * s),
            "shorts" => (0.48, 0.0, 0.010 * s),
            _ => (0.0, 0.0, 0.0),
        };
        let leg = Bone {
            a: hip,
            ra: d.r_thigh,
            b: knee,
            rb: d.r_knee,
            skin,
            cloth,
            cover: thigh_cover,
            loose: leg_loose,
            flare,
            band: None,
            bias: 0.0,
        };
        bone(&mut out, cam, &leg);
        bone(&mut out, cam, &Bone { a: knee, ra: d.r_knee, b: ankle, rb: d.r_ankle, cover: shin_cover, bias: 0.01, ..leg });

        let fp = |z: f64, y: f64| rig.foot_point(i, z, y);
        let shoe = tone(look.shoe_color, far);
        let mut upper = vec![(fp(0.0, 0.004), d.r_ankle + 0.006 * s)];
        let mut sole = Vec::new();
        for sd in SIGN {
            let fc = |z: f64, y: f64, x: f64| rig.foot_corner(i, z, y, sd * x);
            upper.extend([
                (fc(-0.050, -0.058, 0.026), 0.012 * s),
                (fc(0.166, -0.058, 0.022), 0.012 * s),
                (fc(0.135, -0.036, 0.014), 0.030 * s),
                (fc(-0.018, -0.026, 0.010), 0.036 * s),
            ]);
            sole.extend([(fc(SOLE_BACK.0, SOLE_BACK.1, 0.034), SOLE_R * s), (fc(SOLE_FRONT.0, SOLE_FRONT.1, 0.030), SOLE_R * s)]);
        }
        let upper = Shape::hull(&upper, shoe);
        let sole = Shape::hull(&sole, tone(WHITE, far));
        let mut shoe_shapes = vec![sole, upper];
        // Cadarço: um traço claro no peito do pé, só quando o pé não está de costas.
        if cam.view(rig.foot_fwd[i]).z > -0.45 {
            let lace = [fp(0.040, -0.004), fp(0.100, -0.024)];
            shoe_shapes.push(Shape::curve(&lace, 0.011 * s, false, shoe.mix(WHITE, 0.55)));
        }
        out.push(Prim::blob(shoe_shapes, &[ankle, fp(0.14, -0.05)], 0.14));
    }

    // Tronco em dois fechos, peito e quadril, que se encontram na cintura.
    // Roupa justa acompanha a cintura do corpo; roupa solta cai reta a partir
    // do ponto mais largo (`drape`), então o mesmo corpo veste diferente.
    let loose = o.loose * s;
    let hem = rig.spine * (0.035 * s - o.hem * s);
    let inset = |i: usize| rig.shoulder[i] - rig.side * (SIGN[i] * 0.018 * s);
    let chest_half = d.shoulder_half - 0.018 * s + d.r_chest;
    let hip_half = d.hip_half + d.r_hip;
    let gap = (chest_half.max(hip_half) - (d.waist_half + d.r_mid)).max(0.0);
    let mid_r = d.r_mid + loose + gap * o.drape;
    let hip_r = d.r_hip + loose + (chest_half - hip_half).max(0.0) * o.drape * 0.6;
    let waist_disc = |sg: f64| (rig.waist + rig.side * (sg * d.waist_half), mid_r);
    let corner = 0.028 * s;
    let hem_mid = rig.pelvis + hem - rig.spine * (hip_r - corner);
    let chest_part = vec![
        (inset(0), d.r_chest + loose),
        (inset(1), d.r_chest + loose),
        // Trapézio: dá queda aos ombros em vez de um topo reto.
        (rig.chest + rig.spine * (0.012 * s), 0.058 * s + loose),
        (rig.chest - rig.spine * (d.torso * 0.24) + rig.fwd * (0.024 * s), d.r_bust + loose),
        waist_disc(1.0),
        waist_disc(-1.0),
    ];
    let mut hip_part = vec![
        waist_disc(1.0),
        waist_disc(-1.0),
        (rig.hip[0] + rig.spine * (0.05 * s), hip_r),
        (rig.hip[1] + rig.spine * (0.05 * s), hip_r),
        (rig.pelvis + rig.spine * (0.12 * s) + rig.fwd * (0.014 * s), d.r_belly + loose + gap * o.drape * 0.5),
    ];
    // Barra reta: cantos pequenos em volta do quadril, em vez de um fundo redondo.
    for sg in SIGN {
        hip_part.push((hem_mid + rig.side * (sg * (d.hip_half + hip_r - corner)), corner));
        hip_part.push((hem_mid + rig.fwd * (sg * (hip_r + 0.012 * s - corner)), corner));
    }
    let band: Vec<(V3, f64)> =
        hip_part[5..].iter().flat_map(|&(c, r)| [(c - rig.spine * (0.004 * s), r), (c + rig.spine * (0.03 * s), r)]).collect();
    let mut torso = vec![Shape::union(vec![chest_part, hip_part], look.top_color).shaded(shading)];
    if o.ribbed {
        torso.push(Shape::hull(&band, look.top_color.mix(INK, 0.13)));
    }
    let facing = cam.view(rig.fwd).z;
    if facing > 0.5 {
        let front =
            |up: f64, side: f64, out: f64| rig.chest + rig.spine * (d.torso * (up - 1.0)) + rig.side * (side * s) + rig.fwd * (out * s);
        if o.lapel {
            torso.push(Shape::poly(&[front(1.0, 0.055, 0.07), front(1.0, -0.055, 0.07), front(0.56, 0.0, 0.11)], 0.004 * s, WHITE));
        }
        if o.collar {
            for sg in SIGN {
                torso.push(Shape::poly(
                    &[front(1.02, 0.0, 0.08), front(1.03, sg * 0.062, 0.06), front(0.93, sg * 0.035, 0.09)],
                    0.004 * s,
                    look.top_color.light(),
                ));
            }
        }
        if o.pocket {
            torso.push(Shape::poly(
                &[front(0.36, 0.08, 0.10), front(0.36, -0.08, 0.10), front(0.20, -0.10, 0.10), front(0.20, 0.10, 0.10)],
                0.014 * s,
                look.top_color.mix(INK, 0.12),
            ));
        }
    }
    out.push(Prim::blob(torso, &[rig.shoulder[0], rig.shoulder[1], rig.hip[0], rig.hip[1]], 0.12));

    if bottom == "skirt" {
        let t = o.skirt.unwrap_or(0.62);
        let color = if dress { look.top_color } else { look.bottom_color };
        let hem_at = |i: usize| rig.hip[i].lerp(rig.knee[i], t) + rig.side * (SIGN[i] * 0.02 * s);
        let waist = rig.spine * (0.03 * s);
        let key = [rig.hip[0], rig.hip[1], hem_at(0), hem_at(1)];
        // Barra reta que acompanha as pernas: cantos pequenos em volta das coxas.
        let mid = hem_at(0).lerp(hem_at(1), 0.5);
        let across = (hem_at(0) - hem_at(1)) * 0.5;
        let drop = (mid - rig.pelvis).norm();
        let depth_axis = across.cross(drop).norm();
        let flare = d.r_thigh + 0.055 * s;
        let corner = 0.02 * s;
        let mut discs = vec![(rig.hip[0] + waist, d.r_hip + loose), (rig.hip[1] + waist, d.r_hip + loose)];
        for sg in SIGN {
            discs.push((mid + across * sg + across.norm() * (sg * (flare - corner)), corner));
            discs.push((mid + depth_axis * (sg * (flare - corner)), corner));
        }
        let shape = Shape::hull(&discs, color).shaded(shading);
        out.push(Prim::blob(vec![shape], &key, 0.10));
    }

    if o.hood {
        let c = rig.chest - rig.fwd * (0.085 * s) + rig.spine * (0.035 * s);
        out.push(Prim::blob(
            vec![Shape::hull(&[(c, 0.082 * s), (c - rig.spine * (0.06 * s), 0.07 * s)], look.top_color.mix(INK, 0.10))],
            &[c],
            0.13,
        ));
    }

    for i in 0..2 {
        let far = is_far(&rig.shoulder, i);
        let skin = tone(look.skin, far);
        // O braço de cá fica um tom mais claro que o tronco, para não se
        // fundir com ele quando passa na frente da mesma cor.
        let near = is_far(&rig.shoulder, 1 - i);
        let cloth = if near { look.top_color.mix(WHITE, 0.07) } else { tone(look.top_color, far) };
        let (sh, el, wr) = (rig.shoulder[i], rig.elbow[i], rig.wrist[i]);
        let ls = loose + 0.004 * s;

        let long = o.sleeve >= 1.0;
        let arm = Bone {
            a: sh,
            ra: d.r_arm,
            b: el,
            rb: d.r_elbow,
            skin,
            cloth,
            cover: o.sleeve.min(1.0),
            loose: ls,
            flare: 0.006 * s,
            band: None,
            bias: 0.16,
        };
        bone(&mut out, cam, &arm);
        bone(
            &mut out,
            cam,
            &Bone {
                a: el,
                ra: d.r_elbow,
                b: wr,
                rb: d.r_wrist,
                cover: if long { 0.9 } else { 0.0 },
                flare: 0.004 * s,
                band: o.ribbed.then(|| cloth.mix(INK, 0.13)),
                bias: 0.17,
                ..arm
            },
        );

        let dir = (wr - el).norm();
        let hand = hand_shapes(rig, i, skin);
        out.push(Prim::blob(hand, &[wr, wr + dir * (0.1 * s)], 0.18));
    }

    if look.accessories.iter().any(|a| a == "backpack") {
        let c = rig.chest - rig.fwd * (0.15 * s) - rig.spine * (0.10 * s);
        let color = Rgb::hex(0xffc043);
        let color = if color.distance(look.top_color) < 60.0 { Rgb::hex(0x3f6bff) } else { color };
        out.push(Prim::blob(
            vec![
                Shape::hull(&[(c, 0.10 * s), (c - rig.spine * (0.2 * s), 0.115 * s)], color),
                Shape::hull(&[(c - rig.spine * (0.2 * s) - rig.fwd * (0.05 * s), 0.07 * s)], color.mix(INK, 0.14)),
            ],
            &[c, c - rig.spine * (0.2 * s)],
            0.11,
        ));
    }

    out.extend(head::draw(rig, look, cam));
    out
}

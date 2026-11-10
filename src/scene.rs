//! Monta a cena no mundo: resolve posições (`at`, `near`), apoios (`on`),
//! assentos (`sits_on`), contatos de mão (`uses`), olhares e apontamentos,
//! e devolve as peças para desenhar.
use crate::camera::Camera;
use crate::color::{self, Rgb, Style, INK};
use crate::figure::draw::{self, Look};
use crate::figure::pose::{self, Grip, Pose};
use crate::figure::{self, motion, Dims, Placement, Rig, SOLE_BACK};
use crate::math::{v3, V3};
use crate::prim::Prim;
use crate::props::{self, Xf};
use crate::rng::Rng;
use crate::spec::{Item, Spec, SKIN_MODES};
use serde_json::json;
use std::collections::HashMap;
use std::f64::consts::TAU;
use std::ops::Range;

/// O que a cena decidiu sobre um item, para `explain` e para os avisos.
pub struct ItemInfo {
    pub index: usize,
    pub prims: Range<usize>,
    pub pos: V3,
    /// Itens com os quais este tem contato de propósito (assento, apoio, uso).
    pub partners: Vec<usize>,
    pub resolved: serde_json::Value,
}

pub struct Scene {
    pub prims: Vec<Prim>,
    /// Sombras de contato: polígonos no chão.
    pub shadows: Vec<Vec<V3>>,
    /// Plataforma sob o grupo.
    pub ground: Vec<V3>,
    pub rigs: Vec<Rig>,
    pub warnings: Vec<String>,
    pub items: Vec<ItemInfo>,
}

struct Placed {
    xf: Xf,
    /// Altura local da superfície de apoio.
    top: f64,
    anchors: Vec<(String, V3)>,
}

fn ellipse(c: V3, rx: f64, rz: f64, yaw: f64) -> Vec<V3> {
    (0..96)
        .map(|i| {
            let a = TAU * i as f64 / 96.0;
            v3(c.x, 0.0, c.z) + v3(rx * a.cos(), 0.0, rz * a.sin()).rot_y(yaw)
        })
        .collect()
}

fn name_hash(s: &str) -> usize {
    s.bytes().fold(7usize, |h, b| h.wrapping_mul(31).wrapping_add(b as usize)) % 97
}

/// Escolhe a aparência do personagem: o que o JSON não disser é sorteado
/// pela semente, com cores que não brigam com a pele.
fn look_for(it: &Item, index: usize, cast: usize, style: &Style, rng: &mut Rng) -> (Look, String) {
    let skin = match it.skin.as_deref() {
        Some(s) if !SKIN_MODES.contains(&s) => color::resolve_skin(s).unwrap_or(INK),
        Some("real") => color::SKIN_REAL[(index * 2 + cast) % 5].1,
        _ => color::SKIN_FANTASY[(index * 2 + cast) % 5].1,
    };
    let clear_of = |c: Rgb, others: &[Rgb]| others.iter().all(|o| c.distance(*o) > 105.0);
    let n = style.accents.len();
    let start = index * 2 + cast + rng.below(2);
    let top_color = it
        .top_color
        .as_deref()
        .and_then(color::resolve)
        .unwrap_or_else(|| (0..n).map(|k| style.accent(start + k)).find(|c| clear_of(*c, &[skin])).unwrap_or(style.accent(start)));
    let bottom_color = it.bottom_color.as_deref().and_then(color::resolve).unwrap_or_else(|| style.bottoms()[rng.below(4)]);
    let shoe_color = it.shoe_color.as_deref().and_then(color::resolve).unwrap_or_else(|| {
        let first = rng.below(n + 1);
        (0..=n)
            .map(|k| if (first + k) % (n + 1) == n { color::CREAM } else { style.accent(first + k) })
            .find(|c| clear_of(*c, &[bottom_color, skin]))
            .unwrap_or(color::CREAM)
    });
    let hair = it.hair.clone().unwrap_or_else(|| draw::HAIRS[(index * 3 + cast) % 7].to_string());
    let hair_color = it.hair_color.as_deref().and_then(color::resolve_hair).unwrap_or_else(|| {
        let first = rng.below(4);
        (0..6).map(|k| color::HAIR_COLORS[(first + k) % 6].1).find(|c| clear_of(*c, &[skin])).unwrap_or(INK)
    });
    let outfit_name = it.outfit.clone().unwrap_or_else(|| draw::OUTFITS[(index + cast * 2 + rng.below(2)) % 5].to_string());
    let look = Look {
        skin,
        hair,
        hair_color,
        outfit: draw::outfit(&outfit_name).unwrap_or(draw::outfit("tshirt").unwrap()),
        top_color,
        bottom: it.bottom.clone().unwrap_or_else(|| "pants".into()),
        bottom_color,
        shoe_color,
        face: it.face.clone().unwrap_or_else(|| "none".into()),
        beard: it.beard.unwrap_or(false),
        accessories: it.accessories.clone(),
    };
    (look, outfit_name)
}

/// Mistura a pose pedida com a de sentar: pernas e quadril do assento,
/// braços e cabeça da pose original.
fn seated_version(p: Pose) -> Pose {
    if p.seated {
        return Pose { floor: false, ..p };
    }
    let sit = pose::pose("sitting").unwrap();
    Pose { hands: p.hands, grips: p.grips, head_yaw: p.head_yaw, head_pitch: p.head_pitch, tilt: p.tilt, ..sit }
}

/// Ângulo (radianos) para olhar de `from` para `to` no plano do chão.
fn heading(from: V3, to: V3) -> f64 {
    (to.x - from.x).atan2(to.z - from.z)
}

pub fn build(spec: &Spec, cam: &Camera, style: &Style, time: f64) -> Result<Scene, String> {
    let seed = spec.seed.unwrap_or(1);
    let items = &spec.items;
    let n = items.len();
    let mut scene = Scene { prims: vec![], shadows: vec![], ground: vec![], rigs: vec![], warnings: vec![], items: vec![] };
    let index_of: HashMap<&str, usize> = items.iter().enumerate().filter_map(|(i, it)| it.id.as_deref().map(|id| (id, i))).collect();
    let find = |id: &str| index_of.get(id).copied();
    let key = |i: usize| {
        let it = &items[i];
        match &it.who {
            Some(who) => format!("who:{who}"),
            None => format!("{}:{}", it.kind, it.id.clone().unwrap_or_else(|| i.to_string())),
        }
    };

    // Quanto cada item ocupa no chão.
    let foot: Vec<(f64, f64)> = items
        .iter()
        .map(|it| {
            let k = it.scale.unwrap_or(1.0);
            let base = match it.kind.as_str() {
                "person" => (0.26, 0.26),
                "custom" => (0.3, 0.3),
                kind => match spec.defs.get(kind) {
                    Some(def) => def.shadow.map(|s| (s[0], s[1])).unwrap_or((0.3, 0.3)),
                    None => props::footprint(kind).unwrap_or((0.15, 0.15)),
                },
            };
            (base.0 * k, base.1 * k)
        })
        .collect();

    // Posições no chão. Quem não diz onde fica é enfileirado na ordem do JSON.
    let free = |it: &Item| it.on.is_none() && it.sits_on.is_none();
    let loose: Vec<usize> =
        (0..n).filter(|&i| free(&items[i]) && items[i].at.is_none() && items[i].near.is_none() && items[i].kind != "card").collect();
    let mut pos: Vec<Option<V3>> = (0..n)
        .map(|i| {
            let it = &items[i];
            if !free(it) || it.near.is_some() {
                return None;
            }
            Some(match it.at {
                Some([x, z]) => v3(x, 0.0, z),
                None => {
                    let slot = loose.iter().position(|&j| j == i).unwrap_or(0) as f64;
                    v3((slot - (loose.len() as f64 - 1.0) / 2.0) * 1.05, 0.0, 0.0)
                }
            })
        })
        .collect();
    // Ponto do chão de qualquer item, seguindo apoios e assentos até a base.
    fn base_of(i: usize, items: &[Item], pos: &[Option<V3>], find: &dyn Fn(&str) -> Option<usize>, depth: usize) -> Option<V3> {
        if depth > 16 {
            return None;
        }
        if let Some(p) = pos[i] {
            return Some(p);
        }
        let parent = items[i].on.as_deref().or(items[i].sits_on.as_deref())?;
        base_of(find(parent)?, items, pos, find, depth + 1)
    }
    for _ in 0..n {
        let mut moved = false;
        for i in 0..n {
            let Some(near) = &items[i].near else { continue };
            if pos[i].is_some() {
                continue;
            }
            let Some(t) = find(&near.id) else { continue };
            let Some(base) = base_of(t, items, &pos, &find, 0) else { continue };
            let gap = near.gap.unwrap_or(0.15);
            let (dir, reach) = match near.side.as_deref().unwrap_or("right") {
                "left" => (v3(-1.0, 0.0, 0.0), foot[t].0 + foot[i].0),
                "front" => (v3(0.0, 0.0, 1.0), foot[t].1 + foot[i].1),
                "behind" => (v3(0.0, 0.0, -1.0), foot[t].1 + foot[i].1),
                _ => (v3(1.0, 0.0, 0.0), foot[t].0 + foot[i].0),
            };
            pos[i] = Some(base + dir * (reach + gap));
            moved = true;
        }
        if !moved {
            break;
        }
    }
    if let Some(i) = (0..n).find(|&i| items[i].near.is_some() && pos[i].is_none()) {
        return Err(format!("{}: near forma um ciclo; ancore um dos itens com at", items[i].label(i)));
    }
    let yaw_of = |i: usize, own: V3| -> f64 {
        let it = &items[i];
        let turn = it.facing.unwrap_or(0.0).to_radians();
        match it.face_to.as_deref().and_then(find).and_then(|t| base_of(t, items, &pos, &find, 0)) {
            Some(target) if (target - own).len() > 1e-6 => heading(own, target) + turn,
            _ => turn,
        }
    };

    let mut extent = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
    let mut grow = |p: V3, r: f64| {
        extent = [extent[0].min(p.x - r), extent[1].min(p.z - r), extent[2].max(p.x + r), extent[3].max(p.z + r)];
    };
    let mut placed: HashMap<usize, Placed> = HashMap::new();
    let mut focus: Vec<Option<V3>> = vec![None; n];
    let mut infos: Vec<Option<ItemInfo>> = (0..n).map(|_| None).collect();

    // Objetos: cada passada constrói os que já têm onde se apoiar.
    let mut waiting: Vec<usize> = (0..n).filter(|&i| items[i].kind != "person").collect();
    while !waiting.is_empty() {
        let before = waiting.len();
        let mut later = Vec::new();
        for i in waiting {
            let it = &items[i];
            let k = it.scale.unwrap_or(1.0);
            let xf = match it.on.as_deref().and_then(find) {
                None => {
                    let p = pos[i].unwrap_or_default();
                    let default_lift = if it.kind == "card" { 1.5 } else { 0.0 };
                    Xf { pos: p + v3(0.0, it.lift.unwrap_or(default_lift), 0.0), yaw: yaw_of(i, p), k }
                }
                Some(parent) => {
                    let Some(base) = placed.get(&parent) else {
                        later.push(i);
                        continue;
                    };
                    let [x, z] = it.at.unwrap_or([0.0, 0.0]);
                    Xf {
                        pos: base.xf.at(x, base.top + it.lift.unwrap_or(0.0), z),
                        yaw: base.xf.yaw + it.facing.unwrap_or(0.0).to_radians(),
                        k,
                    }
                }
            };
            let mut rng = Rng::derive(seed, &key(i));
            let color = it.color.as_deref().and_then(color::resolve);
            let content = it.content.as_deref().unwrap_or(if it.kind == "board" { "chart" } else { "ui" });
            let inline = it.parts.as_ref().map(|parts| crate::spec::CustomDef { parts: parts.clone(), ..Default::default() });
            let built = match inline.as_ref().or(spec.defs.get(&it.kind)) {
                Some(def) => props::build_custom(def, xf, cam, style, color, &mut rng),
                None => match props::build(&it.kind, xf, cam, style, color, content, &mut rng) {
                    Some(b) => b,
                    None => continue,
                },
            };
            if let (Some((rx, rz)), true) = (built.shadow, it.on.is_none() && xf.pos.y < 0.05) {
                scene.shadows.push(ellipse(xf.pos, rx * k, rz * k, xf.yaw));
                grow(xf.pos, rx.max(rz) * k);
            }
            let keys: Vec<V3> = built.prims.iter().flat_map(|p| p.key.iter().copied()).collect();
            focus[i] =
                Some(if keys.is_empty() { xf.pos } else { keys.iter().fold(V3::default(), |a, b| a + *b) * (1.0 / keys.len() as f64) });
            let start = scene.prims.len();
            scene.prims.extend(built.prims);
            infos[i] = Some(ItemInfo {
                index: i,
                prims: start..scene.prims.len(),
                pos: xf.pos,
                partners: it.on.as_deref().and_then(find).into_iter().collect(),
                resolved: json!({ "facing": (xf.yaw.to_degrees() * 10.0).round() / 10.0, "scale": k }),
            });
            placed.insert(i, Placed { xf, top: built.top, anchors: built.anchors });
        }
        if later.len() == before {
            let i = later[0];
            return Err(format!("{}: on forma um ciclo ou aponta para um personagem", items[i].label(i)));
        }
        waiting = later;
    }

    // Personagens, em duas etapas: primeiro onde cada um está (para que um
    // possa olhar para o outro), depois a pose.
    struct Actor {
        i: usize,
        place: Placement,
        dims: Dims,
        look: Look,
        outfit: String,
        pose_name: String,
        pose: Pose,
        seat_of: Option<usize>,
    }
    let cast = Rng::derive(seed, "cast").below(5);
    let mut actors: Vec<Actor> = Vec::new();
    for (i, it) in items.iter().enumerate().filter(|(_, it)| it.kind == "person") {
        let who = it.label(i);
        let mut rng = Rng::derive(seed, &key(i));
        let (index, offset) = match &it.who {
            Some(name) => (name_hash(name), 0),
            None => (actors.len(), cast),
        };
        let (look, outfit) = look_for(it, index, offset, style, &mut rng);
        let height = it.height.unwrap_or_else(|| 1.72 * rng.range(0.95, 1.05));
        let dims = Dims::new(height * it.scale.unwrap_or(1.0), it.build.as_deref().unwrap_or("regular"));
        let default_pose = match (&it.sits_on, &it.uses) {
            (Some(_), Some(_)) => "typing",
            (Some(_), None) => "sitting",
            _ => "standing",
        };
        let pose_name = it.pose.clone().unwrap_or_else(|| default_pose.to_string());
        let mut p = match it.motion.as_deref() {
            Some(m) => motion::sample(m, time).ok_or_else(|| format!("{who}: motion desconhecido"))?,
            None => pose::pose(&pose_name).ok_or_else(|| format!("{who}: pose desconhecida"))?,
        };
        let (place, seat_of) = match it.sits_on.as_deref().and_then(find) {
            Some(s) => {
                let id = it.sits_on.as_deref().unwrap_or_default();
                let seat = placed.get(&s).ok_or_else(|| format!("{who}: '{id}' não é um objeto"))?;
                let name = it.seat.map(|n| format!("seat{n}"));
                let anchor = seat
                    .anchors
                    .iter()
                    .find(|(a, _)| name.as_ref().map(|n| a == n).unwrap_or(a.starts_with("seat")))
                    .ok_or_else(|| format!("{who}: '{id}' não tem assento{}", if name.is_some() { " com esse número" } else { "" }))?;
                p = seated_version(p);
                let place = Placement {
                    pos: v3(anchor.1.x, 0.0, anchor.1.z),
                    facing: seat.xf.yaw + it.facing.unwrap_or(0.0).to_radians(),
                    seat: Some(anchor.1.y),
                };
                (place, Some(s))
            }
            None => {
                if p.seated && !p.floor {
                    return Err(format!(
                        "{who}: a pose '{pose_name}' precisa de sits_on com o id de um assento (chair, stool, sofa, ball)"
                    ));
                }
                let at = pos[i].unwrap_or_default();
                let facing = if it.face_to.is_some() {
                    yaw_of(i, at)
                } else {
                    let toward_center = if at.x < -0.2 {
                        1.0
                    } else if at.x > 0.2 {
                        -1.0
                    } else {
                        0.45
                    };
                    let moving =
                        matches!(pose_name.as_str(), "walking" | "running") || matches!(it.motion.as_deref(), Some("walk" | "run"));
                    it.facing.unwrap_or(if moving { 72.0 } else { 28.0 } * toward_center).to_radians()
                };
                (Placement { pos: at, facing, seat: None }, None)
            }
        };
        let eye_level = match place.seat {
            Some(h) => h + 0.74 * dims.s,
            None if p.floor => 0.7 * dims.s,
            None => 1.56 * dims.s,
        };
        focus[i] = Some(place.pos + v3(0.0, eye_level, 0.0));
        actors.push(Actor { i, place, dims, look, outfit, pose_name, pose: p, seat_of });
    }

    for actor in actors {
        let Actor { i, place, dims, look, outfit, pose_name, pose: mut p, seat_of } = actor;
        let it = &items[i];
        let who = it.label(i);
        let s = dims.s;
        let to_local = |w: V3| (w - place.pos).rot_y(-place.facing);
        let mut partners: Vec<usize> = seat_of.into_iter().collect();

        if let Some(target) = it.look_at.as_deref().and_then(find).and_then(|t| focus[t]) {
            let d = to_local(target) - to_local(focus[i].unwrap_or(place.pos));
            p.head_yaw = d.x.atan2(d.z).to_degrees().clamp(-75.0, 75.0);
            p.head_pitch = (-d.y).atan2(d.x.hypot(d.z)).to_degrees().clamp(-35.0, 35.0);
        }

        let mut reach = [None, None];
        if let Some(t) = it.uses.as_deref().and_then(find) {
            // Quem usa o laptop também está, de propósito, junto da mesa onde ele fica.
            let mut link = Some(t);
            while let Some(l) = link {
                partners.push(l);
                link = items[l].on.as_deref().and_then(find).filter(|p| !partners.contains(p));
            }
            let anchors = placed.get(&t).map(|pl| pl.anchors.as_slice()).unwrap_or(&[]);
            let anchor = |name: &str| anchors.iter().find(|(a, _)| a == name).map(|(_, p)| *p);
            let side = v3(1.0, 0.0, 0.0).rot_y(place.facing);
            if let Some(a) = anchor("use") {
                reach = [Some(a + side * 0.085), Some(a - side * 0.085)];
            } else if let Some(a) = anchor("touch") {
                let hand = if to_local(a).x > 0.0 { 0 } else { 1 };
                reach[hand] = Some(a);
            } else {
                scene
                    .warnings
                    .push(format!("{who}: '{}' não tem ponto de uso; as mãos ficaram na pose", it.uses.as_deref().unwrap_or_default()));
            }
            if let Some(target) = focus[t] {
                if to_local(target).z < -0.15 {
                    scene.warnings.push(format!(
                        "{who}: está de costas para '{}'; gire com facing ou use face_to",
                        it.uses.as_deref().unwrap_or_default()
                    ));
                }
            }
        }

        let mut rig = figure::solve(&dims, &p, &place, reach);
        if let Some(target) = it.points_at.as_deref().and_then(find).and_then(|t| focus[t]) {
            // O braço do lado do alvo estica na direção dele.
            let hand = if to_local(target).x > 0.0 { 0 } else { 1 };
            let dir = (target - rig.shoulder[hand]).norm();
            reach[hand] = Some(rig.shoulder[hand] + dir * ((dims.upper_arm + dims.forearm) * 0.97));
            p.grips[hand] = Grip::Point;
            rig = figure::solve(&dims, &p, &place, reach);
        }
        if rig.strained {
            scene.warnings.push(format!("{who}: a mão ou o pé não alcança o alvo; aproxime o personagem do objeto"));
        }
        let start = scene.prims.len();
        scene.prims.extend(draw::draw(&rig, &look, cam, style.shading));

        if let Some(what) = it.holds.as_deref() {
            let (at, yaw) = if what == "box" {
                (rig.hand(0).lerp(rig.hand(1), 0.5) + rig.fwd * (0.06 * s) + v3(0.0, 0.04 * s, 0.0), place.facing)
            } else {
                (rig.hand(1) + rig.fwd * (0.02 * s) + v3(0.0, 0.01 * s, 0.0), place.facing)
            };
            let accent =
                (0..style.accents.len()).map(|k| style.accent(k)).find(|c| c.distance(look.top_color) > 105.0).unwrap_or(style.accent(0));
            scene.prims.extend(props::held(what, Xf { pos: at, yaw, k: s }, cam, accent));
        }

        // Sombra de contato sob cada pé apoiado; no salto, uma só sob o corpo.
        let mut grounded = false;
        for f in 0..2 {
            let mid = rig.foot_point(f, 0.06, SOLE_BACK.1);
            if mid.y < 0.06 {
                scene.shadows.push(ellipse(mid, 0.085 * s, 0.17 * s, place.facing));
                grounded = true;
            }
            grow(mid, 0.2);
        }
        if !grounded || p.floor {
            scene.shadows.push(ellipse(rig.pelvis.lerp(rig.chest, 0.3), 0.22 * s, 0.14 * s, place.facing));
        }
        infos[i] = Some(ItemInfo {
            index: i,
            prims: start..scene.prims.len(),
            pos: place.pos,
            partners,
            resolved: json!({
                "pose": it.motion.as_ref().map(|m| format!("motion:{m}")).unwrap_or(pose_name),
                "facing": (place.facing.to_degrees() * 10.0).round() / 10.0,
                "height": (dims.s * 172.0).round() / 100.0,
                "outfit": outfit,
                "hair": look.hair,
                "skin": look.skin.css(),
                "top_color": look.top_color.css(),
                "bottom_color": look.bottom_color.css(),
                "shoe_color": look.shoe_color.css(),
                "hair_color": look.hair_color.css(),
            }),
        });
        scene.rigs.push(rig);
    }

    // Dois itens soltos no mesmo pedaço de chão quase sempre é engano.
    for a in 0..n {
        for b in a + 1..n {
            let (Some(pa), Some(pb)) = (pos[a], pos[b]) else { continue };
            if items[a].kind == "card" || items[b].kind == "card" {
                continue;
            }
            let reach = (foot[a].0.min(foot[a].1) + foot[b].0.min(foot[b].1)) * 0.55;
            if (pa - pb).len() < reach {
                scene.warnings.push(format!(
                    "{} e {} ocupam o mesmo lugar no chão; afaste com at ou use near",
                    items[a].label(a),
                    items[b].label(b)
                ));
            }
        }
    }

    if extent[0] < extent[2] {
        let c = v3((extent[0] + extent[2]) / 2.0, 0.0, (extent[1] + extent[3]) / 2.0);
        let rx = (extent[2] - extent[0]) / 2.0 * 1.12 + 0.35;
        let rz = ((extent[3] - extent[1]) / 2.0 * 1.12 + 0.3).max(rx * 0.28);
        scene.ground = ellipse(c, rx, rz, 0.0);
    }
    scene.items = infos.into_iter().flatten().collect();
    Ok(scene)
}

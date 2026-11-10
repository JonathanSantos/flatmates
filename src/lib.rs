//! Flatmates: ilustrações Corporate Memphis em SVG a partir de um JSON.
//!
//! O núcleo é puro: `render` recebe o texto do JSON e devolve o SVG.
//! Ler arquivo, gravar e rasterizar ficam na borda (`main.rs`, `raster.rs`).
pub mod backdrop;
pub mod camera;
pub mod color;
pub mod figure;
pub mod math;
pub mod prim;
pub mod props;
#[cfg(feature = "png")]
pub mod raster;
pub mod rng;
pub mod scene;
pub mod sort;
pub mod spec;
pub mod text;
#[cfg(target_arch = "wasm32")]
pub mod wasm;

use camera::Camera;
use color::{Style, Theme, INK};
use math::V3;
use prim::{num, Geom, Prim, Projector, Shape};
use rng::Rng;
use scene::Scene;
use serde_json::json;
use sort::SortItem;
use spec::{CameraSpec, Spec};
use std::fmt::Write;

pub use spec::parse;

#[derive(Default, Clone, Copy)]
pub struct Options {
    /// Desenha o esqueleto por cima dos personagens.
    pub rig: bool,
    /// Instante do ciclo de movimento, de 0 a 1.
    pub time: f64,
    /// Troca a semente do JSON (variações em lote).
    pub seed: Option<u64>,
    /// Troca o tamanho do JSON (mesma cena em outro formato).
    pub size: Option<[u32; 2]>,
}

pub struct Output {
    pub svg: String,
    pub warnings: Vec<String>,
    /// Onde cada item foi parar e o que foi decidido por ele.
    pub explain: serde_json::Value,
}

pub const EXAMPLES: [(&str, &str); 13] = [
    ("team", include_str!("../examples/team.json")),
    ("remote-work", include_str!("../examples/remote-work.json")),
    ("office", include_str!("../examples/office.json")),
    ("isometric", include_str!("../examples/isometric.json")),
    ("lounge", include_str!("../examples/lounge.json")),
    ("pets", include_str!("../examples/pets.json")),
    ("poses", include_str!("../examples/poses.json")),
    ("wardrobe", include_str!("../examples/wardrobe.json")),
    ("custom", include_str!("../examples/custom.json")),
    ("brand", include_str!("../examples/brand.json")),
    ("night", include_str!("../examples/night.json")),
    ("walk", include_str!("../examples/walk.json")),
    ("builds", include_str!("../examples/builds.json")),
];

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn camera_of(spec: &Spec) -> Camera {
    match &spec.camera {
        Some(CameraSpec::Preset(name)) => Camera::preset(name).unwrap_or(Camera::degrees(0.0, 9.0)),
        Some(CameraSpec::Custom { yaw, pitch }) => Camera::degrees(*yaw, *pitch),
        None => Camera::degrees(0.0, 9.0),
    }
}

/// Cena pronta para enquadrar e pintar.
struct Stage {
    spec: Spec,
    w: f64,
    h: f64,
    seed: u64,
    th: Theme,
    style: Style,
    cam: Camera,
    scene: Scene,
    boxes: Vec<[f64; 4]>,
    title: Vec<String>,
    title_size: f64,
}

fn merge(a: [f64; 4], b: [f64; 4]) -> [f64; 4] {
    [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])]
}

const EMPTY: [f64; 4] = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];

impl Stage {
    fn build(spec: Spec, opts: &Options) -> Result<Stage, String> {
        let [w, h] = opts.size.or(spec.size).unwrap_or([1200, 630]).map(|v| v as f64);
        let seed = opts.seed.or(spec.seed).unwrap_or(1);
        let palette = spec.palette.as_ref().map(|p| p.iter().filter_map(|c| color::resolve(c)).collect::<Vec<_>>());
        let named = color::theme(spec.theme.as_deref().unwrap_or("day")).ok_or("tema desconhecido")?;
        let th = match spec.background.as_deref().and_then(color::Rgb::parse) {
            Some(bg) => color::theme_from_bg(bg, palette.as_deref().unwrap_or(&[])),
            None => named,
        };
        let style = Style::new(palette, th.dark, spec.shading.unwrap_or(true));
        let cam = camera_of(&spec);
        let seeded = Spec { seed: Some(seed), ..spec };
        let scene = scene::build(&seeded, &cam, &style, opts.time)?;
        let boxes: Vec<[f64; 4]> = scene.prims.iter().map(|p| p.bounds(&cam)).collect();
        let base = h * 0.082;
        let (title, title_size) = match seeded.title.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
            Some(t) => text::fit(t, base, w * 0.84),
            None => (vec![], base),
        };
        Ok(Stage { spec: seeded, w, h, seed, th, style, cam, scene, boxes, title, title_size })
    }

    /// Caixa da cena em espaço de vista (metros).
    fn bounds(&self) -> [f64; 4] {
        self.boxes.iter().fold(EMPTY, |a, b| merge(a, *b))
    }

    fn title_bottom(&self) -> f64 {
        if self.title.is_empty() {
            return 0.0;
        }
        self.h * 0.145 + (self.title.len() as f64 - 1.0) * self.title_size * 1.14
    }

    /// Enquadramento automático: a caixa `bb` inteira cabe na área útil.
    fn projector(&self, bb: [f64; 4]) -> Projector {
        let (w, h) = (self.w, self.h);
        let bb = if bb[0] > bb[2] { [-1.0, 0.0, 1.0, 1.7] } else { bb };
        let top = if self.title.is_empty() { h * 0.10 } else { (self.title_bottom() + h * 0.10).min(h * 0.5) };
        let region = [w * 0.09, top, w * 0.91, h * 0.90];
        let (bw, bh) = ((bb[2] - bb[0]).max(0.2), (bb[3] - bb[1]).max(0.2));
        let scale = ((region[2] - region[0]) / bw).min((region[3] - region[1]) / bh).min((region[3] - region[1]) / 1.5);
        Projector {
            cam: self.cam,
            scale,
            ox: (region[0] + region[2]) / 2.0 - (bb[0] + bb[2]) / 2.0 * scale,
            oy: (region[1] + region[3]) / 2.0 + (bb[1] + bb[3]) / 2.0 * scale,
        }
    }
}

/// Corta em dois um tubo (peça de duas juntas) no ponto `t` do eixo dele.
/// Cada forma é repartida pelos discos de cada lado do corte, com um disco
/// novo no corte para as duas metades continuarem emendadas.
fn cut_tube(p: &Prim, t: f64) -> Option<[Prim; 2]> {
    let (a, b) = (p.key[0], p.key[1]);
    let axis = b - a;
    let len2 = axis.dot(axis).max(1e-12);
    let along = |q: V3| (q - a).dot(axis) / len2;
    let cut = a.lerp(b, t);
    let mut halves: [Vec<Shape>; 2] = [Vec::new(), Vec::new()];
    for shape in &p.shapes {
        let Geom::Hull(discs) = &shape.geom else { return None };
        let us: Vec<f64> = discs.iter().map(|d| along(d.0)).collect();
        let (lo, hi) = (us.iter().cloned().fold(f64::MAX, f64::min), us.iter().cloned().fold(f64::MIN, f64::max));
        if hi <= t + 1e-6 {
            halves[0].push(shape.clone());
            continue;
        }
        if lo >= t - 1e-6 {
            halves[1].push(shape.clone());
            continue;
        }
        // Raio de cada ponta: um disco só, ou a meia-largura de uma ponta reta.
        let end_radius = |u_end: f64| {
            let at_end: Vec<&(V3, f64)> = discs.iter().zip(&us).filter(|(_, u)| (**u - u_end).abs() < 1e-6).map(|(d, _)| d).collect();
            let c = at_end.iter().fold(V3::default(), |s, d| s + d.0) * (1.0 / at_end.len() as f64);
            at_end.iter().map(|d| (d.0 - c).len() + d.1).fold(0.0, f64::max)
        };
        let r = end_radius(lo) + (end_radius(hi) - end_radius(lo)) * ((t - lo) / (hi - lo));
        for (side, keep) in [(0, true), (1, false)] {
            let mut part: Vec<(V3, f64)> = discs.iter().zip(&us).filter(|(_, u)| (**u < t) == keep).map(|(d, _)| *d).collect();
            part.push((cut, r));
            let mut piece = shape.clone();
            piece.geom = Geom::Hull(part);
            halves[side].push(piece);
        }
    }
    let [first, second] = halves;
    Some([Prim::blob(first, &[a, cut], p.bias), Prim::blob(second, &[cut, b], p.bias)])
}

/// Desfaz ciclos de oclusão dividindo as peças presas: tubos são cortados
/// no plano da face que atravessam, e faces retangulares grandes viram quatro.
fn split_stuck(prims: &mut Vec<Prim>, stuck: &[usize]) -> usize {
    let planes: Vec<(V3, V3)> = stuck.iter().filter_map(|&i| prims[i].normal.map(|n| (prims[i].key[0], n))).collect();
    let mut split = 0;
    for &i in stuck {
        let p = &prims[i];
        if p.normal.is_none() && p.key.len() == 2 {
            let (a, b) = (p.key[0], p.key[1]);
            // Primeiro plano que o eixo do tubo atravessa com folga.
            let hit = planes.iter().find_map(|(p0, n)| {
                let (da, db) = (n.dot(a - *p0), n.dot(b - *p0));
                let t = da / (da - db);
                (da * db < 0.0 && (0.12..=0.88).contains(&t)).then_some(t)
            });
            if let Some([first, second]) = hit.and_then(|t| cut_tube(p, t)) {
                prims[i] = first;
                prims.push(second);
                split += 1;
            }
            continue;
        }
        let (Some(normal), [shape]) = (p.normal, p.shapes.as_slice()) else { continue };
        let Geom::Poly { pts, round } = &shape.geom else { continue };
        if pts.len() != 4 || (pts[0] - pts[2]).len() < 0.5 {
            continue;
        }
        let (fill, round) = (shape.fill, *round);
        let q = [pts[0], pts[1], pts[2], pts[3]];
        let mid = |a: V3, b: V3| a.lerp(b, 0.5);
        let c = mid(q[0], q[2]);
        let m = [mid(q[0], q[1]), mid(q[1], q[2]), mid(q[2], q[3]), mid(q[3], q[0])];
        let quads = [[q[0], m[0], c, m[3]], [m[0], q[1], m[1], c], [c, m[1], q[2], m[2]], [m[3], c, m[2], q[3]]];
        for (k, quad) in quads.iter().enumerate() {
            let piece = Prim::face(vec![Shape::poly(quad, round, fill)], quad, normal);
            if k == 0 {
                prims[i] = piece;
            } else {
                prims.push(piece);
            }
        }
        split += 1;
    }
    split
}

fn sort_items(prims: &[Prim], cam: &Camera) -> Vec<SortItem> {
    prims
        .iter()
        .map(|p| {
            let pts: Vec<V3> = p.key.iter().map(|k| cam.view(*k)).collect();
            let depth = pts.iter().map(|v| v.z).sum::<f64>() / pts.len().max(1) as f64 + p.bias;
            SortItem {
                pts,
                normal: p.normal.map(|n| cam.view(n)),
                depth,
                bbox: p.bounds(cam),
                outline: p.outline(cam),
                ceiling: p.ceiling.map(|(q, n)| (cam.view(q), cam.view(n))),
            }
        })
        .collect()
}

/// As três partes do SVG: o que fica igual entre quadros de uma animação
/// vai em `open` e `close`; o que muda vai em `body`.
struct Painted {
    open: String,
    body: String,
    close: String,
    warnings: Vec<String>,
    explain: serde_json::Value,
}

/// `shared` é a caixa comum a todos os quadros de uma animação: mantém o
/// fundo igual de um quadro para o outro.
fn paint(mut st: Stage, pj: &Projector, opts: &Options, shared: Option<[f64; 4]>) -> Painted {
    let (w, h, seed) = (st.w, st.h, st.seed);
    let bb = shared.unwrap_or(st.bounds());
    let bb = if bb[0] > bb[2] { [-1.0, 0.0, 1.0, 1.7] } else { bb };
    let px = |b: [f64; 4]| [pj.ox + b[0] * pj.scale, pj.oy - b[3] * pj.scale, pj.ox + b[2] * pj.scale, pj.oy - b[1] * pj.scale];
    let area = backdrop::Area { w, h, focus: px(bb), title_bottom: st.title_bottom() };
    let mut warnings = std::mem::take(&mut st.scene.warnings);

    // Caixa de cada item em pixels, para o `explain` e os avisos de composição.
    let item_box: Vec<[f64; 4]> =
        st.scene.items.iter().map(|it| px(st.boxes[it.prims.clone()].iter().fold(EMPTY, |a, b| merge(a, *b)))).collect();
    let item_depth: Vec<f64> = st
        .scene
        .items
        .iter()
        .map(|it| {
            let keys: Vec<f64> = st.scene.prims[it.prims.clone()].iter().flat_map(|p| p.key.iter().map(|k| st.cam.view(*k).z)).collect();
            keys.iter().sum::<f64>() / keys.len().max(1) as f64
        })
        .collect();
    // Névoa de profundidade: em cenas com fundo de verdade, cada item puxa
    // para a cor do fundo conforme a distância. Separa os planos sem contorno.
    let (near, far) = item_depth.iter().fold((f64::MIN, f64::MAX), |(n, f), d| (n.max(*d), f.min(*d)));
    if st.spec.haze.unwrap_or(true) && near - far > 1.2 {
        for (info, depth) in st.scene.items.iter().zip(&item_depth) {
            let fade = ((near - depth) / (near - far)).powf(1.3) * 0.16;
            for prim in &mut st.scene.prims[info.prims.clone()] {
                for shape in &mut prim.shapes {
                    shape.fill = shape.fill.mix(st.th.bg, fade);
                }
            }
        }
    }
    let label = |n: usize| st.spec.items[st.scene.items[n].index].label(st.scene.items[n].index);
    let mut small = false;
    for (a, ia) in st.scene.items.iter().enumerate() {
        if st.spec.items[ia.index].kind != "person" {
            continue;
        }
        let ba = item_box[a];
        let size = ((ba[2] - ba[0]) * (ba[3] - ba[1])).max(1.0);
        small |= ba[3] - ba[1] < h * 0.2;
        for (b, ib) in st.scene.items.iter().enumerate() {
            if a == b || ia.partners.contains(&ib.index) || ib.partners.contains(&ia.index) || item_depth[b] <= item_depth[a] {
                continue;
            }
            let bbx = item_box[b];
            let (ox, oy) = ((ba[2].min(bbx[2]) - ba[0].max(bbx[0])).max(0.0), (ba[3].min(bbx[3]) - ba[1].max(bbx[1])).max(0.0));
            if ox * oy / size > 0.8 {
                warnings.push(format!("{} fica quase todo atrás de {}; mova um dos dois com at", label(a), label(b)));
            }
        }
    }
    if small {
        warnings.push("a cena está muito espalhada e os personagens ficaram pequenos; aproxime os itens".into());
    }

    // Ordem de pintura por oclusão; se houver ciclo, divide as faces grandes e tenta de novo.
    let (mut order, mut stuck) = sort::order(&sort_items(&st.scene.prims, &st.cam));
    for _ in 0..4 {
        if stuck.is_empty() || split_stuck(&mut st.scene.prims, &stuck) == 0 {
            break;
        }
        (order, stuck) = sort::order(&sort_items(&st.scene.prims, &st.cam));
    }
    if !stuck.is_empty() {
        warnings.push(format!(
            "ordem de pintura ambígua entre {} peças que se atravessam; afaste os itens que estão um dentro do outro",
            stuck.len()
        ));
    }

    let th = st.th;
    let mut open = String::with_capacity(8 * 1024);
    let _ = write!(open, "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {w} {h}\" width=\"{w}\" height=\"{h}\">");
    let _ = write!(open, "<rect width=\"{w}\" height=\"{h}\" fill=\"{}\"/>", th.bg.css());
    let on = |flag: Option<bool>| flag.unwrap_or(true);
    let bd = &st.spec.backdrop;
    open.push_str("<g id=\"backdrop\">");
    if on(bd.blob) {
        open.push_str(&backdrop::blobs(&area, &th, &mut Rng::derive(seed, "blobs")));
    }
    if on(bd.shapes) {
        open.push_str(&backdrop::shapes(&area, &st.style, &mut Rng::derive(seed, "shapes")));
    }
    if on(bd.confetti) {
        open.push_str(&backdrop::confetti(&area, &st.style, &mut Rng::derive(seed, "confetti")));
    }
    open.push_str("</g>");
    if on(bd.ground) && !st.scene.ground.is_empty() {
        let _ = write!(open, "<g id=\"ground\">{}</g>", Shape::poly(&st.scene.ground, 0.0, th.ground).svg(pj));
    }

    let mut body = String::with_capacity(48 * 1024);
    if !st.scene.shadows.is_empty() {
        let _ = write!(body, "<g opacity=\"{}\">", if th.dark { 0.34 } else { 0.14 });
        let shadow = if th.dark { color::Rgb::hex(0x0a0c22) } else { INK };
        for s in &st.scene.shadows {
            body.push_str(&Shape::poly(s, 0.0, shadow).svg(pj));
        }
        body.push_str("</g>");
    }
    body.push_str("<g>");
    for i in order {
        for shape in &st.scene.prims[i].shapes {
            body.push_str(&shape.svg(pj));
        }
    }
    body.push_str("</g>");
    if opts.rig {
        body.push_str("<g stroke=\"#ff00aa\" stroke-width=\"2\" fill=\"#ffffff\">");
        for rig in &st.scene.rigs {
            for (a, b) in rig.bones() {
                let (a, b) = (pj.p(a), pj.p(b));
                let _ = write!(body, "<path d=\"M{} {}L{} {}\"/>", num(a.x), num(a.y), num(b.x), num(b.y));
                let _ = write!(body, "<circle cx=\"{}\" cy=\"{}\" r=\"3\"/>", num(b.x), num(b.y));
            }
        }
        body.push_str("</g>");
    }

    let mut close = String::new();
    for (k, line) in st.title.iter().enumerate() {
        let _ = write!(
            close,
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"Inter, 'Helvetica Neue', Arial, sans-serif\" font-weight=\"700\" font-size=\"{}\" fill=\"{}\">{}</text>",
            num(w / 2.0),
            num(h * 0.145 + k as f64 * st.title_size * 1.14),
            num(st.title_size),
            st.style.text().css(),
            escape(line)
        );
    }
    if bd.grain.unwrap_or(false) {
        let _ = write!(
            close,
            "<filter id=\"grain\" x=\"0\" y=\"0\" width=\"100%\" height=\"100%\"><feTurbulence type=\"fractalNoise\" baseFrequency=\"0.85\" numOctaves=\"2\" seed=\"{}\"/><feColorMatrix values=\"0 0 0 0 0.12 0 0 0 0 0.14 0 0 0 0 0.27 0 0 0 -1.6 1.05\"/></filter><rect width=\"{w}\" height=\"{h}\" filter=\"url(#grain)\" opacity=\"0.16\"/>",
            seed % 997
        );
    }
    close.push_str("</svg>\n");

    let round = |v: f64| (v * 10.0).round() / 10.0;
    let items: Vec<serde_json::Value> = st
        .scene
        .items
        .iter()
        .enumerate()
        .map(|(n, info)| {
            let it = &st.spec.items[info.index];
            let mut v = json!({
                "index": info.index,
                "id": it.id,
                "kind": it.kind,
                "position": [round(info.pos.x * 10.0) / 10.0, round(info.pos.z * 10.0) / 10.0],
                "box": item_box[n].map(round),
            });
            if let (Some(obj), Some(extra)) = (v.as_object_mut(), info.resolved.as_object()) {
                obj.extend(extra.clone());
            }
            v
        })
        .collect();
    let explain = json!({
        "size": [w, h],
        "seed": seed,
        "camera": { "yaw": round(st.cam.yaw.to_degrees()), "pitch": round(st.cam.pitch.to_degrees()) },
        "pixels_per_meter": round(pj.scale),
        "group_box": area.focus.map(round),
        "items": items,
        "warnings": warnings,
    });
    Painted { open, body, close, warnings, explain }
}

pub fn render(json: &str, opts: &Options) -> Result<Output, String> {
    let st = Stage::build(parse(json)?, opts)?;
    let pj = st.projector(st.bounds());
    let p = paint(st, &pj, opts, None);
    Ok(Output { svg: format!("{}{}{}", p.open, p.body, p.close), warnings: p.warnings, explain: p.explain })
}

/// Monta os quadros de um ciclo com o mesmo enquadramento em todos.
fn staged_frames(json: &str, count: usize, opts: &Options) -> Result<(Vec<Painted>, Vec<String>), String> {
    if !(2..=120).contains(&count) {
        return Err("frames vai de 2 a 120".into());
    }
    let spec = parse(json)?;
    let stages = (0..count)
        .map(|k| Stage::build(spec.clone(), &Options { time: k as f64 / count as f64, ..*opts }))
        .collect::<Result<Vec<_>, _>>()?;
    let mut stages = stages;
    let bb = stages.iter().fold(EMPTY, |a, s| merge(a, s.bounds()));
    // O chão do primeiro quadro vale para todos: o fundo não pode pulsar.
    let ground = stages[0].scene.ground.clone();
    stages.iter_mut().for_each(|s| s.scene.ground = ground.clone());
    let pj = stages[0].projector(bb);
    let mut warnings: Vec<String> = Vec::new();
    if spec.items.iter().all(|it| it.motion.is_none()) {
        warnings.push("nenhum item tem motion; todos os quadros saem iguais".into());
    }
    let painted: Vec<Painted> = stages.into_iter().map(|s| paint(s, &pj, opts, Some(bb))).collect();
    for p in &painted {
        for w in &p.warnings {
            if !warnings.contains(w) {
                warnings.push(w.clone());
            }
        }
    }
    Ok((painted, warnings))
}

/// Um SVG por quadro, todos com o mesmo enquadramento.
pub fn frames(json: &str, count: usize, opts: &Options) -> Result<Vec<Output>, String> {
    let (painted, warnings) = staged_frames(json, count, opts)?;
    Ok(painted
        .into_iter()
        .map(|p| Output { svg: format!("{}{}{}", p.open, p.body, p.close), warnings: warnings.clone(), explain: p.explain })
        .collect())
}

/// Um único SVG animado: os quadros se revezam, como num flipbook.
pub fn flipbook(json: &str, count: usize, fps: f64, opts: &Options) -> Result<Output, String> {
    let (painted, warnings) = staged_frames(json, count, opts)?;
    let dur = count as f64 / fps.clamp(1.0, 60.0);
    let mut svg = painted[0].open.clone();
    for (k, p) in painted.iter().enumerate() {
        let (a, b) = (k as f64 / count as f64, (k + 1) as f64 / count as f64);
        let _ = write!(
            svg,
            "<g display=\"{}\">{}<animate attributeName=\"display\" calcMode=\"discrete\" dur=\"{dur:.3}s\" repeatCount=\"indefinite\" keyTimes=\"0;{a:.4};{b:.4};1\" values=\"none;inline;none;none\"/></g>",
            if k == 0 { "inline" } else { "none" },
            p.body
        );
    }
    svg.push_str(&painted[0].close);
    let explain = painted.into_iter().next().map(|p| p.explain).unwrap_or_default();
    Ok(Output { svg, warnings, explain })
}

#[cfg(test)]
mod tests {
    use super::*;
    use math::v3;

    #[test]
    fn cut_tube_keeps_both_ends_and_meets_at_the_cut() {
        let (a, b) = (v3(0.0, 0.0, 0.0), v3(0.0, 1.0, 0.0));
        let skin = Shape::hull(&[(a, 0.10), (b, 0.06)], INK);
        // Manga com ponta reta que cobre só 60% do osso.
        let sleeve = Shape::hull(&[(a, 0.12), (v3(0.07, 0.6, 0.0), 0.01), (v3(-0.07, 0.6, 0.0), 0.01)], INK);
        let tube = Prim::blob(vec![skin, sleeve], &[a, b], 0.17);
        let [low, high] = cut_tube(&tube, 0.4).expect("corta");
        assert_eq!((low.key[0], high.key[1]), (a, b));
        assert_eq!(low.key[1], high.key[0]);
        assert_eq!((low.shapes.len(), high.shapes.len(), low.bias), (2, 2, 0.17));
        let Geom::Hull(d) = &low.shapes[0].geom else { panic!() };
        // Raio no corte: interpolado entre 0,10 e 0,06.
        assert!((d.last().unwrap().1 - 0.084).abs() < 1e-9);
        // Um corte depois do fim da manga deixa a manga inteira na primeira metade.
        let [low, high] = cut_tube(&tube, 0.8).expect("corta");
        assert_eq!((low.shapes.len(), high.shapes.len()), (2, 1));
    }

    #[test]
    fn split_stuck_cuts_a_tube_that_crosses_a_face() {
        let quad = [v3(-0.1, 0.5, -0.1), v3(0.1, 0.5, -0.1), v3(0.1, 0.5, 0.1), v3(-0.1, 0.5, 0.1)];
        let face = Prim::face(vec![Shape::poly(&quad, 0.0, INK)], &quad, v3(0.0, 1.0, 0.0));
        let (a, b) = (v3(0.0, 0.0, 0.0), v3(0.0, 1.0, 0.0));
        let tube = Prim::blob(vec![Shape::hull(&[(a, 0.05), (b, 0.05)], INK)], &[a, b], 0.0);
        let mut prims = vec![face, tube];
        assert_eq!(split_stuck(&mut prims, &[0, 1]), 1);
        assert_eq!(prims.len(), 3);
        assert!((prims[1].key[1].y - 0.5).abs() < 1e-9 && (prims[2].key[0].y - 0.5).abs() < 1e-9);
    }
}

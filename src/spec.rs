//! Contrato do JSON de entrada e validação com mensagens que listam as
//! opções aceitas.
use crate::camera;
use crate::color;
use crate::figure::draw::{ACCESSORIES, BOTTOMS, FACES, HAIRS, OUTFITS};
use crate::figure::motion::MOTIONS;
use crate::figure::pose::POSES;
use crate::figure::BUILDS;
use crate::props::{CONTENTS, HOLDS, KINDS};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    /// Largura e altura em pixels. Padrão: 1200 x 630.
    pub size: Option<[u32; 2]>,
    pub theme: Option<String>,
    /// Cor de fundo própria (hex); os blobs e o chão derivam dela.
    pub background: Option<String>,
    /// Cores de marca que substituem os acentos padrão (2 a 8 hex).
    pub palette: Option<Vec<String>>,
    /// Sombra tonal nas formas grandes. Padrão: ligada.
    pub shading: Option<bool>,
    /// Névoa de profundidade: o que está mais ao fundo puxa de leve para a
    /// cor do fundo. Padrão: ligada.
    pub haze: Option<bool>,
    pub camera: Option<CameraSpec>,
    pub seed: Option<u64>,
    pub title: Option<String>,
    #[serde(default)]
    pub backdrop: Backdrop,
    /// Personagens nomeados, reutilizáveis com `who`.
    #[serde(default)]
    pub cast: BTreeMap<String, serde_json::Value>,
    /// Objetos próprios: o nome vira um `kind`.
    #[serde(default)]
    pub defs: BTreeMap<String, CustomDef>,
    #[serde(default)]
    pub items: Vec<Item>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum CameraSpec {
    Preset(String),
    Custom { yaw: f64, pitch: f64 },
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Backdrop {
    pub blob: Option<bool>,
    pub shapes: Option<bool>,
    pub confetti: Option<bool>,
    pub ground: Option<bool>,
    pub grain: Option<bool>,
}

/// Objeto descrito pela IA com formas simples, em metros. O motor cuida de
/// oclusão, três tons, sombra no chão e estilo.
#[derive(Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct CustomDef {
    pub parts: Vec<Part>,
    /// Altura da superfície onde outros objetos se apoiam (`on`).
    pub top: Option<f64>,
    /// Ponto de assento [x, y, z].
    pub seat: Option<[f64; 3]>,
    /// Ponto onde as duas mãos trabalham (`uses`).
    #[serde(rename = "use")]
    pub use_point: Option<[f64; 3]>,
    /// Ponto que uma mão toca (`uses`).
    pub touch: Option<[f64; 3]>,
    /// Meia-largura e meia-profundidade da sombra no chão.
    pub shadow: Option<[f64; 2]>,
}

/// Coordenadas locais em metros: x direita, y cima, z frente.
#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "shape", rename_all = "lowercase", deny_unknown_fields)]
pub enum Part {
    /// `at` é o centro.
    Box {
        at: [f64; 3],
        size: [f64; 3],
        color: Option<String>,
        round: Option<f64>,
        yaw: Option<f64>,
    },
    /// `at` é o centro da base; `r_top` faz um tronco de cone.
    Cylinder {
        at: [f64; 3],
        r: f64,
        r_top: Option<f64>,
        h: f64,
        color: Option<String>,
    },
    Sphere {
        at: [f64; 3],
        r: f64,
        color: Option<String>,
    },
    /// Fecho convexo de esferas [x, y, z, raio]: formas orgânicas.
    Hull {
        points: Vec<[f64; 4]>,
        color: Option<String>,
    },
    /// Tubo suave que passa pelas esferas [x, y, z, raio].
    Chain {
        points: Vec<[f64; 4]>,
        color: Option<String>,
    },
    /// Painel plano em pé; `at` é o centro, `size` é [largura, altura].
    Panel {
        at: [f64; 3],
        size: [f64; 2],
        content: Option<String>,
        color: Option<String>,
        yaw: Option<f64>,
    },
}

impl Part {
    pub fn color(&self) -> &Option<String> {
        match self {
            Part::Box { color, .. }
            | Part::Cylinder { color, .. }
            | Part::Sphere { color, .. }
            | Part::Hull { color, .. }
            | Part::Chain { color, .. }
            | Part::Panel { color, .. } => color,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Near {
    pub id: String,
    /// left, right, front ou behind (do ponto de vista da câmera frontal).
    pub side: Option<String>,
    /// Folga em metros entre os dois.
    pub gap: Option<f64>,
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Item {
    pub id: Option<String>,
    pub kind: String,
    /// Posição no chão em metros: [x para a direita, z em direção à câmera].
    pub at: Option<[f64; 2]>,
    /// Posição relativa a outro item, em vez de `at`.
    pub near: Option<Near>,
    /// Graus: 0 olha para a câmera frontal, 90 para a direita, -90 para a esquerda.
    pub facing: Option<f64>,
    /// Vira o item de frente para outro (id).
    pub face_to: Option<String>,
    /// Id do objeto em cima do qual este fica.
    pub on: Option<String>,
    /// Altura acima do chão, em metros.
    pub lift: Option<f64>,
    pub scale: Option<f64>,
    pub color: Option<String>,
    pub content: Option<String>,
    /// Só para `kind: "custom"`: as formas do objeto.
    pub parts: Option<Vec<Part>>,
    // Só para `person`:
    /// Nome de um personagem de `cast`.
    pub who: Option<String>,
    pub pose: Option<String>,
    /// Movimento cíclico, usado por `flatmates animate`.
    pub motion: Option<String>,
    pub sits_on: Option<String>,
    pub seat: Option<usize>,
    pub uses: Option<String>,
    pub holds: Option<String>,
    /// A cabeça vira para este item (id).
    pub look_at: Option<String>,
    /// Um braço aponta para este item (id).
    pub points_at: Option<String>,
    pub outfit: Option<String>,
    pub bottom: Option<String>,
    pub top_color: Option<String>,
    pub bottom_color: Option<String>,
    pub shoe_color: Option<String>,
    pub hair: Option<String>,
    pub hair_color: Option<String>,
    pub skin: Option<String>,
    pub face: Option<String>,
    pub beard: Option<bool>,
    #[serde(default)]
    pub accessories: Vec<String>,
    pub build: Option<String>,
    pub height: Option<f64>,
}

impl Item {
    pub fn label(&self, index: usize) -> String {
        match &self.id {
            Some(id) => format!("'{id}'"),
            None => format!("items[{index}] ({})", self.kind),
        }
    }
}

pub const SKIN_MODES: [&str; 2] = ["fantasy", "real"];
pub const SIDES: [&str; 4] = ["left", "right", "front", "behind"];
pub const PART_ROLES: [&str; 3] = ["accent", "accent2", "steel"];

/// Lê o JSON, aplicando antes os personagens de `cast` aos itens com `who`:
/// o que o item disser vale por cima do elenco.
pub fn parse(json: &str) -> Result<Spec, String> {
    let mut root: serde_json::Value = serde_json::from_str(json).map_err(|e| format!("JSON inválido: {e}"))?;
    let cast = root.get("cast").and_then(|c| c.as_object()).cloned().unwrap_or_default();
    if let Some(items) = root.get_mut("items").and_then(|i| i.as_array_mut()) {
        for (n, item) in items.iter_mut().enumerate() {
            let Some(who) = item.get("who").and_then(|w| w.as_str()).map(str::to_string) else { continue };
            let Some(base) = cast.get(&who).and_then(|b| b.as_object()) else {
                let names: Vec<&str> = cast.keys().map(String::as_str).collect();
                return Err(format!("items[{n}]: who '{who}' não está em cast. Elenco: {}", names.join(", ")));
            };
            let Some(obj) = item.as_object_mut() else { continue };
            for (k, v) in base {
                obj.entry(k.clone()).or_insert_with(|| v.clone());
            }
        }
    }
    let spec: Spec = serde_json::from_value(root).map_err(|e| format!("JSON inválido: {e}"))?;
    validate(&spec)?;
    Ok(spec)
}

fn one_of(what: &str, value: &str, options: &[&str]) -> Result<(), String> {
    if options.contains(&value) {
        return Ok(());
    }
    Err(format!("{what} '{value}' não existe. Opções: {}", options.join(", ")))
}

fn opt(what: &str, value: &Option<String>, options: &[&str]) -> Result<(), String> {
    match value {
        Some(v) => one_of(what, v, options),
        None => Ok(()),
    }
}

fn color_ok(what: &str, value: &Option<String>, resolve: fn(&str) -> Option<color::Rgb>, names: Vec<&str>) -> Result<(), String> {
    let Some(v) = value else { return Ok(()) };
    if resolve(v).is_some() {
        return Ok(());
    }
    Err(format!("{what} '{v}' não existe. Use um hex (#rrggbb) ou: {}", names.join(", ")))
}

fn names(t: &[(&'static str, color::Rgb)]) -> Vec<&'static str> {
    t.iter().map(|(n, _)| *n).collect()
}

fn parts_ok(who: &str, parts: &[Part]) -> Result<(), String> {
    if parts.is_empty() || parts.len() > 40 {
        return Err(format!("{who}: um objeto próprio tem de 1 a 40 formas"));
    }
    for (n, p) in parts.iter().enumerate() {
        let here = format!("{who}, forma {n}");
        if let Some(c) = p.color() {
            if !PART_ROLES.contains(&c.as_str()) && color::resolve(c).is_none() {
                let mut all = PART_ROLES.to_vec();
                all.extend(names(&color::NAMED));
                return Err(format!("{here}: color '{c}' não existe. Use um hex ou: {}", all.join(", ")));
            }
        }
        let ok = match p {
            Part::Box { size, .. } => size.iter().all(|v| (0.001..=6.0).contains(v)),
            Part::Cylinder { r, r_top, h, .. } => {
                (0.001..=3.0).contains(r) && (0.001..=6.0).contains(h) && r_top.is_none_or(|t| (0.0..=3.0).contains(&t))
            }
            Part::Sphere { r, .. } => (0.001..=3.0).contains(r),
            Part::Hull { points, .. } | Part::Chain { points, .. } => {
                (1..=24).contains(&points.len()) && points.iter().all(|q| (0.0..=3.0).contains(&q[3]))
            }
            Part::Panel { size, content, .. } => {
                opt("content", content, &CONTENTS).map_err(|e| format!("{here}: {e}"))?;
                size.iter().all(|v| (0.01..=6.0).contains(v))
            }
        };
        if !ok {
            return Err(format!("{here}: medidas fora da faixa (metros; raios até 3, tamanhos até 6)"));
        }
    }
    Ok(())
}

pub fn validate(spec: &Spec) -> Result<(), String> {
    opt("theme", &spec.theme, &color::THEMES)?;
    color_ok("background", &spec.background, color::Rgb::parse, vec![])?;
    if let Some(p) = &spec.palette {
        if !(2..=8).contains(&p.len()) || p.iter().any(|c| color::resolve(c).is_none()) {
            return Err("palette: de 2 a 8 cores, cada uma um hex (#rrggbb) ou um nome de cor".into());
        }
    }
    if let Some(CameraSpec::Preset(name)) = &spec.camera {
        one_of("camera", name, &camera::PRESETS)?;
    }
    if let Some(CameraSpec::Custom { yaw, pitch }) = &spec.camera {
        if !(-80.0..=80.0).contains(yaw) || !(0.0..=75.0).contains(pitch) {
            return Err("camera: yaw vai de -80 a 80 e pitch de 0 a 75 graus".into());
        }
    }
    if let Some([w, h]) = spec.size {
        if !(64..=8192).contains(&w) || !(64..=8192).contains(&h) {
            return Err("size: largura e altura vão de 64 a 8192".into());
        }
    }
    let mut kinds: Vec<&str> = vec!["person", "custom"];
    kinds.extend(KINDS);
    for (name, def) in &spec.defs {
        if kinds.contains(&name.as_str()) {
            return Err(format!("defs: '{name}' já é um kind nativo; escolha outro nome"));
        }
        parts_ok(&format!("defs.{name}"), &def.parts)?;
    }
    kinds.extend(spec.defs.keys().map(String::as_str));
    let ids: Vec<&str> = spec.items.iter().filter_map(|i| i.id.as_deref()).collect();
    for (n, it) in spec.items.iter().enumerate() {
        let who = it.label(n);
        let wrap = |r: Result<(), String>| r.map_err(|e| format!("{who}: {e}"));
        wrap(one_of("kind", &it.kind, &kinds))?;
        if let Some(id) = &it.id {
            if ids.iter().filter(|x| **x == id).count() > 1 {
                return Err(format!("{who}: id repetido"));
            }
        }
        let near_id = it.near.as_ref().map(|n| n.id.clone());
        for (field, target) in [
            ("on", &it.on),
            ("sits_on", &it.sits_on),
            ("uses", &it.uses),
            ("near.id", &near_id),
            ("face_to", &it.face_to),
            ("look_at", &it.look_at),
            ("points_at", &it.points_at),
        ] {
            let Some(t) = target else { continue };
            if !ids.contains(&t.as_str()) {
                return Err(format!("{who}: {field} aponta para '{t}', que não é id de nenhum item"));
            }
            if it.id.as_deref() == Some(t.as_str()) {
                return Err(format!("{who}: {field} aponta para o próprio item"));
            }
        }
        if let Some(near) = &it.near {
            wrap(opt("near.side", &near.side, &SIDES))?;
            if it.at.is_some() || it.on.is_some() || it.sits_on.is_some() {
                return Err(format!("{who}: use só um entre at, near, on e sits_on"));
            }
        }
        wrap(opt("content", &it.content, &CONTENTS))?;
        wrap(color_ok("color", &it.color, color::resolve, names(&color::NAMED)))?;
        if let Some(s) = it.scale {
            if !(0.2..=5.0).contains(&s) {
                return Err(format!("{who}: scale vai de 0.2 a 5"));
            }
        }
        match (&it.parts, it.kind == "custom") {
            (Some(parts), true) => parts_ok(&who, parts)?,
            (None, true) => return Err(format!("{who}: kind 'custom' precisa de parts (ou defina o objeto em defs)")),
            (Some(_), false) => return Err(format!("{who}: parts só vale com kind 'custom'")),
            (None, false) => {}
        }
        if it.kind != "person" {
            continue;
        }
        wrap(opt("pose", &it.pose, &POSES))?;
        wrap(opt("motion", &it.motion, &MOTIONS))?;
        wrap(opt("holds", &it.holds, &HOLDS))?;
        wrap(opt("outfit", &it.outfit, &OUTFITS))?;
        wrap(opt("bottom", &it.bottom, &BOTTOMS))?;
        wrap(opt("hair", &it.hair, &HAIRS))?;
        wrap(opt("face", &it.face, &FACES))?;
        wrap(opt("build", &it.build, &BUILDS))?;
        for a in &it.accessories {
            wrap(one_of("accessory", a, &ACCESSORIES))?;
        }
        wrap(color_ok("top_color", &it.top_color, color::resolve, names(&color::NAMED)))?;
        wrap(color_ok("bottom_color", &it.bottom_color, color::resolve, names(&color::NAMED)))?;
        wrap(color_ok("shoe_color", &it.shoe_color, color::resolve, names(&color::NAMED)))?;
        wrap(color_ok("hair_color", &it.hair_color, color::resolve_hair, names(&color::HAIR_COLORS)))?;
        if let Some(s) = &it.skin {
            if !SKIN_MODES.contains(&s.as_str()) && color::resolve_skin(s).is_none() {
                let mut all = SKIN_MODES.to_vec();
                all.extend(names(&color::SKIN_FANTASY));
                all.extend(names(&color::SKIN_REAL));
                return Err(format!("{who}: skin '{s}' não existe. Use um hex ou: {}", all.join(", ")));
            }
        }
        if let Some(h) = it.height {
            if !(1.0..=2.2).contains(&h) {
                return Err(format!("{who}: height vai de 1.0 a 2.2 metros"));
            }
        }
    }
    Ok(())
}

/// Tudo o que o JSON aceita, para a IA consultar.
pub fn catalog() -> serde_json::Value {
    let kinds: Vec<&str> = ["person", "custom"].into_iter().chain(KINDS).collect();
    serde_json::json!({
        "theme": color::THEMES,
        "background": "#rrggbb (substitui o tema)",
        "palette": "2 a 8 cores de marca",
        "camera": camera::PRESETS,
        "camera_custom": {"yaw": "-80..80", "pitch": "0..75"},
        "kind": kinds,
        "content": CONTENTS,
        "color": names(&color::NAMED),
        "placement": {
            "at": "[x, z] em metros",
            "near": {"id": "id de outro item", "side": SIDES, "gap": "metros"},
            "on": "id do objeto de apoio",
            "face_to": "id para o qual o item se vira",
        },
        "person": {
            "who": "nome em cast",
            "pose": POSES,
            "motion": MOTIONS,
            "outfit": OUTFITS,
            "bottom": BOTTOMS,
            "hair": HAIRS,
            "hair_color": names(&color::HAIR_COLORS),
            "skin": SKIN_MODES.iter().copied().chain(names(&color::SKIN_FANTASY)).chain(names(&color::SKIN_REAL)).collect::<Vec<_>>(),
            "face": FACES,
            "build": BUILDS,
            "accessories": ACCESSORIES,
            "holds": HOLDS,
            "relations": ["sits_on", "uses", "look_at", "points_at"],
        },
        "custom": {
            "shape": ["box", "cylinder", "sphere", "hull", "chain", "panel"],
            "color": PART_ROLES.iter().copied().chain(names(&color::NAMED)).collect::<Vec<_>>(),
            "anchors": ["top", "seat", "use", "touch", "shadow"],
        },
        "backdrop": ["blob", "shapes", "confetti", "ground", "grain"],
    })
}

//! Testes de ponta a ponta: do JSON ao SVG e aos pixels.
use flatmates::camera::{Camera, PRESETS};
use flatmates::color::Style;
use flatmates::scene::Scene;
use flatmates::{flipbook, frames, parse, raster, render, scene, Options, EXAMPLES};
use resvg::tiny_skia::Pixmap;

const PLAIN: &str = r#""haze":false,"backdrop":{"blob":false,"shapes":false,"confetti":false,"ground":false}"#;

fn out(json: &str) -> flatmates::Output {
    render(json, &Options::default()).unwrap_or_else(|e| panic!("deveria renderizar: {e}"))
}

fn svg(json: &str) -> String {
    out(json).svg
}

fn world(json: &str, cam: &str) -> Scene {
    let spec = parse(json).unwrap();
    scene::build(&spec, &Camera::preset(cam).unwrap(), &Style::new(None, false, true), 0.0).unwrap()
}

fn example(name: &str) -> &'static str {
    EXAMPLES.iter().find(|e| e.0 == name).unwrap().1
}

/// Conta pixels de uma cor exata na imagem rasterizada.
fn count(json: &str, rgb: [u8; 3]) -> usize {
    let png = raster::png(&svg(json), 1.0).unwrap();
    let pix = Pixmap::decode_png(&png).unwrap();
    pix.pixels().iter().filter(|p| [p.red(), p.green(), p.blue()] == rgb).count()
}

/// Compara dois SVGs aceitando 0,15 de diferença em cada número: senos e
/// cossenos podem variar no último bit entre arquiteturas.
fn close_enough(a: &str, b: &str) -> bool {
    fn tokens(s: &str) -> Vec<(bool, &str)> {
        let mut out = Vec::new();
        let bytes = s.as_bytes();
        let (mut i, mut start) = (0, 0);
        let numeric = |c: u8| c.is_ascii_digit() || c == b'.' || c == b'-';
        while i < bytes.len() {
            if numeric(bytes[i]) != numeric(bytes[start]) {
                out.push((numeric(bytes[start]), &s[start..i]));
                start = i;
            }
            i += 1;
        }
        out.push((numeric(bytes[start]), &s[start..]));
        out
    }
    let (ta, tb) = (tokens(a), tokens(b));
    ta.len() == tb.len()
        && ta.iter().zip(&tb).all(|((na, xa), (nb, xb))| match (na, nb, xa.parse::<f64>(), xb.parse::<f64>()) {
            (true, true, Ok(x), Ok(y)) => (x - y).abs() <= 0.15,
            _ => xa == xb,
        })
}

#[test]
fn every_example_renders_clean() {
    for (name, json) in EXAMPLES {
        let out = render(json, &Options::default()).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(out.warnings.is_empty(), "{name}: {:?}", out.warnings);
        assert!(out.svg.starts_with("<svg"), "{name}");
        assert!(!out.svg.contains("NaN") && !out.svg.contains("inf"), "{name} tem número inválido");
    }
}

#[test]
fn examples_match_golden_files() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let update = std::env::var("UPDATE_GOLDEN").is_ok();
    for (name, json) in EXAMPLES {
        let path = dir.join(format!("{name}.svg"));
        let got = svg(json);
        if update {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(&path, &got).unwrap();
            continue;
        }
        let want = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("falta {path:?}; rode com UPDATE_GOLDEN=1"));
        assert!(close_enough(&got, &want), "{name} mudou; se foi de propósito, rode com UPDATE_GOLDEN=1");
    }
}

#[test]
fn golden_comparison_tolerates_rounding_but_not_real_changes() {
    assert!(close_enough(r#"<path d="M10 20.1L-3.5 4"/>"#, r#"<path d="M10.1 20L-3.4 4"/>"#));
    assert!(!close_enough(r#"<path d="M10 20"/>"#, r#"<path d="M10 21"/>"#));
    assert!(!close_enough(r#"<path d="M10 20"/>"#, r#"<path d="M10 20" fill="red"/>"#));
}

#[test]
fn same_input_same_bytes_and_seed_changes_output() {
    let a = r#"{"seed":3,"items":[{"kind":"person"},{"kind":"plant"}]}"#;
    let b = r#"{"seed":4,"items":[{"kind":"person"},{"kind":"plant"}]}"#;
    assert_eq!(svg(a), svg(a));
    assert_ne!(svg(a), svg(b));
    // A semente da linha de comando vale por cima da do JSON.
    let forced = render(a, &Options { seed: Some(4), ..Options::default() }).unwrap().svg;
    assert_eq!(forced, svg(b));
}

#[test]
fn errors_list_the_valid_options() {
    let err = |json: &str| render(json, &Options::default()).err().expect("deveria falhar");
    let e = err(r#"{"items":[{"kind":"person","pose":"flying"}]}"#);
    assert!(e.contains("flying") && e.contains("walking"), "{e}");
    let e = err(r#"{"items":[{"kind":"robot"}]}"#);
    assert!(e.contains("chair"), "{e}");
    let e = err(r#"{"items":[{"kind":"person","sits_on":"nada"}]}"#);
    assert!(e.contains("nada"), "{e}");
    let e = err(r#"{"items":[{"kind":"person","pose":"sitting"}]}"#);
    assert!(e.contains("sits_on"), "{e}");
    assert!(err(r#"{"itens":[]}"#).contains("JSON inválido"));
    assert!(err(r#"{"items":[{"kind":"person","top_color":"limao"}]}"#).contains("coral"));
    assert!(err(r#"{"items":[{"kind":"person","who":"ninguem"}]}"#).contains("cast"));
    assert!(err(r#"{"items":[{"kind":"custom"}]}"#).contains("parts"));
    assert!(err(r##"{"palette":["#ff0000"],"items":[]}"##).contains("palette"));
    let e = err(r#"{"items":[{"id":"a","kind":"box","near":{"id":"b"}},{"id":"b","kind":"box","near":{"id":"a"}}]}"#);
    assert!(e.contains("ciclo"), "{e}");
    let e = err(r#"{"items":[{"id":"a","kind":"box","on":"b"},{"id":"b","kind":"box","on":"a"}]}"#);
    assert!(e.contains("ciclo"), "{e}");
}

#[test]
fn seated_person_touches_seat_and_floor() {
    for seat in ["chair", "stool", "sofa", "ball"] {
        let json = format!(r#"{{"items":[{{"id":"s","kind":"{seat}","at":[0.4,0.2],"facing":40}},{{"kind":"person","sits_on":"s"}}]}}"#);
        let sc = world(&json, "front");
        let rig = &sc.rigs[0];
        let seat_h = rig.pelvis.y - rig.d.r_thigh - 0.012 * rig.d.s;
        assert!((0.4..0.7).contains(&seat_h), "{seat}: assento a {seat_h}");
        assert!(sc.warnings.is_empty(), "{seat}: {:?}", sc.warnings);
        let sole = rig.foot_point(0, 0.05, -0.09).y;
        if seat == "stool" {
            // Banco alto: os pés ficam pendurados, com a coxa quase na horizontal.
            assert!(sole > 0.05, "{seat}: sola a {sole}");
            assert!((rig.hip[0].y - rig.knee[0].y).abs() < 0.12, "{seat}: coxa caída");
            continue;
        }
        assert!(sole.abs() < 0.01, "{seat}: sola a {sole} do chão");
    }
}

#[test]
fn hands_land_on_the_laptop_they_use() {
    let sc = world(example("office"), "three-quarter");
    let rig = &sc.rigs[0];
    let gap = (rig.wrist[0] - rig.wrist[1]).len();
    assert!((0.1..0.25).contains(&gap), "pulsos a {gap} um do outro");
    // Tampo da mesa a 0,76 m; as mãos ficam logo acima dele.
    for i in 0..2 {
        assert!((0.76..0.86).contains(&rig.wrist[i].y), "pulso {i} a {}", rig.wrist[i].y);
    }
}

/// Regressão do erro clássico de perspectiva: o que está em cima da mesa
/// aparece inteiro em qualquer câmera, mesmo no fundo do tampo.
#[test]
fn object_on_desk_is_never_hidden_by_the_desk() {
    for cam in PRESETS {
        let json = format!(
            r##"{{"camera":"{cam}",{PLAIN},"items":[{{"id":"d","kind":"desk"}},{{"kind":"mug","on":"d","at":[0.5,-0.3],"color":"#ff00fe","scale":2}}]}}"##
        );
        let seen = count(&json, [255, 0, 254]);
        assert!(seen > 200, "{cam}: caneca com {seen} px");
    }
}

/// Quem está atrás do sofá tem as pernas cobertas por ele.
#[test]
fn sofa_hides_the_legs_behind_it() {
    let scene = |z: f64| {
        format!(
            r##"{{{PLAIN},"shading":false,"items":[{{"kind":"sofa","at":[0,0]}},{{"kind":"person","at":[-0.2,{z}],"facing":0,"bottom_color":"#ff00fe","outfit":"tshirt"}}]}}"##
        )
    };
    let (behind, in_front) = (count(&scene(-0.9), [255, 0, 254]), count(&scene(0.9), [255, 0, 254]));
    assert!(in_front > 1000, "calça visível na frente: {in_front}");
    assert!(behind * 2 < in_front, "atrás do sofá sobraram {behind} px de {in_front}");
}

/// O antebraço de quem digita passa por cima do tampo com o cotovelo
/// embaixo dele: a mão não pode sumir atrás da mesa.
#[test]
fn typing_hands_stay_visible_above_the_desk() {
    // De frente a tampa do laptop cobre as mãos, então o teste usa as câmeras altas.
    for cam in ["three-quarter", "isometric", "high"] {
        let json = format!(
            r##"{{"camera":"{cam}",{PLAIN},"items":[{{"id":"d","kind":"desk"}},{{"id":"l","kind":"laptop","on":"d","at":[0,-0.16],"facing":180}},{{"id":"c","kind":"chair","at":[0,-0.6]}},{{"kind":"person","sits_on":"c","uses":"l","skin":"#ff00fe","outfit":"sweater","hair":"bald"}}]}}"##
        );
        let o = out(&json);
        assert!(o.warnings.is_empty(), "{cam}: {:?}", o.warnings);
        let with_hands = count(&json, [255, 0, 254]);
        let hidden =
            count(&json.replace(r#""uses":"l","#, "").replace(r#""sits_on":"c""#, r#""sits_on":"c","pose":"sitting""#), [255, 0, 254]);
        assert!(with_hands > hidden, "{cam}: mãos na mesa {with_hands} px, no colo {hidden} px");
    }
}

/// O cabelo acompanha a cabeça: de costas não sobra rosto.
#[test]
fn back_view_shows_no_face() {
    let scene = |facing: f64| {
        format!(r##"{{{PLAIN},"items":[{{"kind":"person","facing":{facing},"skin":"#ff00fe","hair":"bob","outfit":"blazer"}}]}}"##)
    };
    let (front, back) = (count(&scene(0.0), [255, 0, 254]), count(&scene(180.0), [255, 0, 254]));
    assert!(front > back, "frente {front}, costas {back}");
}

#[test]
fn custom_object_gets_anchors_shadow_and_roles() {
    let json = r##"{"haze":false,"defs":{"bench":{"parts":[{"shape":"box","at":[0,0.4,0],"size":[1.2,0.08,0.4],"color":"#ff00fe"},{"shape":"cylinder","at":[0,0,0],"r":0.05,"h":0.36,"color":"steel"}],"seat":[0.2,0.44,0],"top":0.44}},
        "items":[{"id":"b","kind":"bench","at":[1,0]},{"kind":"person","sits_on":"b"},{"kind":"mug","on":"b","at":[-0.4,0]}]}"##;
    let sc = world(json, "three-quarter");
    assert!(sc.warnings.is_empty(), "{:?}", sc.warnings);
    let rig = &sc.rigs[0];
    assert!((rig.pelvis.x - 1.2).abs() < 1e-6 && (rig.pelvis.y - (0.44 + rig.d.r_thigh + 0.012 * rig.d.s)).abs() < 1e-6);
    assert_eq!(sc.shadows.len(), 1 + 2, "sombra do banco e dos dois pés");
    assert!(count(json, [255, 0, 254]) > 300, "o tampo do banco aparece na cor pedida");
    // Objeto descrito direto no item, sem defs.
    let inline = r#"{"items":[{"kind":"custom","parts":[{"shape":"sphere","at":[0,0.3,0],"r":0.3},{"shape":"panel","at":[0,1,0],"size":[0.6,0.4],"content":"code"}]}]}"#;
    assert!(out(inline).warnings.is_empty());
}

#[test]
fn near_places_items_side_by_side_and_face_to_turns_them() {
    let json = r#"{"items":[{"id":"d","kind":"desk","at":[1,0.5]},
        {"id":"p","kind":"person","near":{"id":"d","side":"left","gap":0.3},"face_to":"d"},
        {"kind":"plant","near":{"id":"p","side":"behind"}}]}"#;
    let sc = world(json, "front");
    let person = sc.items.iter().find(|i| i.index == 1).unwrap();
    // Mesa ocupa 0,82 para cada lado; a pessoa, 0,26.
    assert!((person.pos.x - (1.0 - 0.82 - 0.26 - 0.3)).abs() < 1e-9, "x = {}", person.pos.x);
    assert!((person.pos.z - 0.5).abs() < 1e-9);
    // De frente para a mesa: olhando para +x.
    assert!(sc.rigs[0].fwd.x > 0.99, "frente {:?}", sc.rigs[0].fwd);
    let plant = sc.items.iter().find(|i| i.index == 2).unwrap();
    assert!(plant.pos.z < person.pos.z - 0.4 && (plant.pos.x - person.pos.x).abs() < 1e-9);
}

#[test]
fn stacked_supports_add_up() {
    let json = r#"{"items":[{"id":"box","kind":"box","at":[0,0]},{"id":"books","kind":"books","on":"box"},{"id":"mug","kind":"mug","on":"books"}]}"#;
    let sc = world(json, "front");
    let y = |n: usize| sc.items.iter().find(|i| i.index == n).unwrap().pos.y;
    assert_eq!(y(0), 0.0);
    assert!((y(1) - 0.38).abs() < 1e-9, "livros a {}", y(1));
    assert!((y(2) - (0.38 + 0.12)).abs() < 1e-9, "caneca a {}", y(2));
}

#[test]
fn look_at_and_points_at_aim_at_the_target() {
    let json = r#"{"items":[{"id":"b","kind":"board","at":[2,0]},{"kind":"person","at":[0,0],"facing":40,"look_at":"b","points_at":"b"}]}"#;
    let sc = world(json, "front");
    let rig = &sc.rigs[0];
    let to_board = (flatmates::math::v3(2.0, 1.2, 0.0) - rig.head).norm();
    assert!(rig.head_fwd.dot(to_board) > 0.9, "cabeça {:?} alvo {:?}", rig.head_fwd, to_board);
    // O braço do lado do alvo fica esticado na direção dele.
    let arm = (0..2).map(|i| (rig.wrist[i] - rig.shoulder[i]).norm().dot(to_board)).fold(f64::MIN, f64::max);
    assert!(arm > 0.93, "braço alinhado em {arm}");
    assert!(sc.warnings.is_empty(), "{:?}", sc.warnings);
}

#[test]
fn one_hand_touches_objects_without_a_keyboard() {
    // Mesa alta (de trabalhar em pé), para a mão alcançar sem agachar.
    let json = r#"{"items":[{"id":"d","kind":"desk","scale":1.35},{"id":"m","kind":"mug","on":"d","at":[0.1,0.25]},{"kind":"person","at":[0.135,0.72],"facing":180,"uses":"m"}]}"#;
    let sc = world(json, "front");
    let rig = &sc.rigs[0];
    let mug = flatmates::math::v3(0.135, 0.76 * 1.35 + 0.06, 0.3375);
    let nearest = (0..2).map(|i| (rig.wrist[i] - mug).len()).fold(f64::MAX, f64::min);
    assert!(nearest < 0.02, "pulso a {nearest} da caneca");
    assert!(sc.warnings.is_empty(), "{:?}", sc.warnings);
}

#[test]
fn cast_keeps_a_character_identical_across_scenes() {
    let scene = |extra: &str, seed: u32| {
        format!(
            r#"{{"seed":{seed},"cast":{{"ana":{{"hair":"bun","outfit":"hoodie"}}}},"items":[{extra}{{"id":"a","kind":"person","who":"ana"}}]}}"#
        )
    };
    let resolved = |json: &str| {
        let o = out(json);
        o.explain["items"].as_array().unwrap().iter().find(|i| i["id"] == "a").unwrap().clone()
    };
    let (one, two) = (resolved(&scene("", 1)), resolved(&scene(r#"{"kind":"person"},{"kind":"plant"},"#, 1)));
    for field in ["skin", "top_color", "hair_color", "shoe_color", "height", "hair", "outfit"] {
        assert_eq!(one[field], two[field], "{field}");
    }
    assert_eq!(one["hair"], "bun");
    // O item pode sobrescrever o elenco.
    let o = out(r#"{"cast":{"ana":{"hair":"bun"}},"items":[{"id":"a","kind":"person","who":"ana","hair":"afro"}]}"#);
    assert_eq!(o.explain["items"][0]["hair"], "afro");
}

#[test]
fn brand_palette_replaces_the_default_accents() {
    let json = r##"{"palette":["#123456","#abcdef","#654321"],"background":"#fafafa","items":[{"kind":"person"},{"kind":"chair"},{"kind":"plant"}]}"##;
    let s = svg(json);
    assert!(s.contains("#123456") || s.contains("#abcdef") || s.contains("#654321"));
    for default in ["#ff6b5b", "#3f6bff", "#8b5cf6", "#ffc043", "#ff6fa8"] {
        assert!(!s.contains(default), "sobrou o acento padrão {default}");
    }
    assert!(s.contains(r##"fill="#fafafa""##));
}

#[test]
fn night_theme_uses_light_text_and_light_trousers() {
    let s = svg(r#"{"theme":"night","title":"Oi","items":[{"kind":"person"}]}"#);
    assert!(s.contains(r##"fill="#f2f4ff">Oi</text>"##), "título claro");
    assert!(!s.contains(r##"fill="#1e2444""##), "nada pintado com a tinta escura sobre fundo escuro");
}

#[test]
fn long_title_wraps_into_lines_and_pushes_the_scene_down() {
    let long = r#"{"title":"Como a nossa equipe distribuída entrega produtos melhores trabalhando de qualquer lugar do mundo","items":[{"id":"p","kind":"person"}]}"#;
    let short = r#"{"title":"Oi","items":[{"id":"p","kind":"person"}]}"#;
    let (l, s) = (out(long), out(short));
    assert!(l.svg.matches("<text").count() >= 2);
    let top = |o: &flatmates::Output| o.explain["items"][0]["box"][1].as_f64().unwrap();
    assert!(top(&l) > top(&s) + 20.0, "longo {} curto {}", top(&l), top(&s));
}

#[test]
fn explain_reports_boxes_inside_the_canvas() {
    let o = out(example("office"));
    let items = o.explain["items"].as_array().unwrap();
    assert_eq!(items.len(), 10);
    for it in items {
        let b: Vec<f64> = it["box"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
        assert!(b[0] >= 0.0 && b[1] >= 0.0 && b[2] <= 1200.0 && b[3] <= 630.0, "{it}");
        assert!(b[2] > b[0] && b[3] > b[1]);
    }
    let person = items.iter().find(|i| i["kind"] == "person").unwrap();
    assert_eq!(person["pose"], "typing");
    assert!(person["top_color"].as_str().unwrap().starts_with('#'));
}

#[test]
fn composition_warnings_point_at_the_problem() {
    let warn = |json: &str| out(json).warnings.join(" | ");
    let w = warn(r#"{"items":[{"id":"a","kind":"person","at":[0,0]},{"id":"b","kind":"plant","at":[0.05,0]}]}"#);
    assert!(w.contains("'a'") && w.contains("'b'") && w.contains("mesmo lugar"), "{w}");
    let w = warn(
        r#"{"items":[{"id":"d","kind":"desk"},{"id":"l","kind":"laptop","on":"d"},{"kind":"person","at":[0,0.8],"facing":0,"uses":"l"}]}"#,
    );
    assert!(w.contains("de costas"), "{w}");
    let w = warn(r#"{"items":[{"id":"s","kind":"sofa","at":[0,0.6],"scale":2},{"id":"p","kind":"person","at":[0,-0.6],"height":1.2}]}"#);
    assert!(w.contains("'p'") && w.contains("atrás"), "{w}");
    let w = warn(r#"{"items":[{"kind":"person","at":[-9,0]},{"kind":"person","at":[9,0]}]}"#);
    assert!(w.contains("espalhada"), "{w}");
    assert!(warn(r#"{"items":[{"kind":"person","at":[-0.6,0]},{"kind":"plant","at":[0.6,0]}]}"#).is_empty());
}

#[test]
fn animation_frames_share_the_frame_and_move() {
    let json = example("walk");
    let all = frames(json, 8, &Options::default()).unwrap();
    assert_eq!(all.len(), 8);
    let head = |s: &str| s[..s.find("<g opacity").unwrap()].to_string();
    assert!(all.iter().all(|f| head(&f.svg) == head(&all[0].svg)), "o fundo muda entre quadros");
    assert!(all.windows(2).all(|w| w[0].svg != w[1].svg), "quadros vizinhos iguais");
    let book = flipbook(json, 8, 12.0, &Options::default()).unwrap();
    assert_eq!(book.svg.matches("<animate ").count(), 8);
    assert!(book.warnings.is_empty(), "{:?}", book.warnings);
    // Sem motion, o aviso diz por que nada se mexe.
    let still = frames(r#"{"items":[{"kind":"person"}]}"#, 4, &Options::default()).unwrap();
    assert!(still[0].warnings.iter().any(|w| w.contains("motion")));
}

#[test]
fn size_override_reframes_the_same_scene() {
    let json = example("team");
    let square = render(json, &Options { size: Some([1080, 1080]), ..Options::default() }).unwrap();
    assert!(square.svg.contains(r#"viewBox="0 0 1080 1080""#));
    let b = square.explain["group_box"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect::<Vec<_>>();
    assert!(b[0] >= 0.0 && b[2] <= 1080.0 && b[3] <= 1080.0, "{b:?}");
}

/// Largura, linha a linha, do que foi pintado com a cor dada.
fn row_widths(json: &str, rgb: [u8; 3]) -> Vec<usize> {
    let png = raster::png(&svg(json), 1.0).unwrap();
    let pix = Pixmap::decode_png(&png).unwrap();
    let w = pix.width() as usize;
    pix.pixels().chunks(w).map(|row| row.iter().filter(|p| [p.red(), p.green(), p.blue()] == rgb).count()).filter(|n| *n > 0).collect()
}

/// A roupa responde ao corpo: a justa marca a cintura, a solta cai reta.
#[test]
fn fitted_clothes_show_the_waist_and_loose_clothes_hide_it() {
    let torso = |build: &str, outfit: &str| {
        let json = format!(
            r##"{{{PLAIN},"shading":false,"items":[{{"kind":"person","build":"{build}","outfit":"{outfit}","pose":"celebrating","facing":0,"top_color":"#ff00fe","hair":"bald"}}]}}"##
        );
        let rows = row_widths(&json, [255, 0, 254]);
        let n = rows.len();
        // Só o tronco: abaixo dos braços erguidos (mangas têm a mesma cor).
        let body = &rows[n * 45 / 100..];
        let m = body.len();
        let waist = *body[m * 15 / 100..m * 55 / 100].iter().min().unwrap() as f64;
        let hip = *body[m * 60 / 100..].iter().max().unwrap() as f64;
        waist / hip
    };
    let (tank, hoodie) = (torso("curvy", "tank"), torso("curvy", "hoodie"));
    assert!(tank < 0.90, "regata justa deveria marcar a cintura: {tank:.2}");
    assert!(hoodie > 0.96, "moletom solto deveria cair reto: {hoodie:.2}");
    // O mesmo moletom em corpos diferentes não tem a mesma largura.
    let width = |build: &str| {
        let json = format!(
            r##"{{{PLAIN},"shading":false,"items":[{{"kind":"person","build":"{build}","outfit":"tank","pose":"celebrating","facing":0,"top_color":"#ff00fe","hair":"bald","height":1.72}}]}}"##
        );
        *row_widths(&json, [255, 0, 254]).iter().max().unwrap()
    };
    let (slim, regular, broad) = (width("slim"), width("regular"), width("broad"));
    assert!(slim < regular && regular < broad, "larguras {slim} {regular} {broad}");
}

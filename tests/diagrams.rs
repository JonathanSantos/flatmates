//! Os diagramas do README precisam ser SVG válido.
use std::path::Path;

#[test]
fn readme_diagrams_are_valid_svg() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/img");
    let mut seen = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("svg") {
            continue;
        }
        let svg = std::fs::read_to_string(&path).unwrap();
        let png = flatmates::raster::png(&svg, 1.0).unwrap_or_else(|e| panic!("{path:?}: {e}"));
        assert!(png.len() > 2000, "{path:?} rasterizou vazio");
        if let Ok(out) = std::env::var("DIAGRAM_PNG_DIR") {
            std::fs::write(Path::new(&out).join(path.with_extension("png").file_name().unwrap()), png).unwrap();
        }
        seen += 1;
    }
    assert!(seen >= 7, "esperava os diagramas em docs/img, achei {seen}");
}

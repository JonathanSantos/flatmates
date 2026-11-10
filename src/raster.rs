//! SVG para PNG com resvg, usando a fonte Inter embutida.
use resvg::{tiny_skia, usvg};

use crate::text::INTER_BOLD;

pub fn png(svg: &str, scale: f32) -> Result<Vec<u8>, String> {
    let mut opt = usvg::Options::default();
    opt.fontdb_mut().load_font_data(INTER_BOLD.to_vec());
    let tree = usvg::Tree::from_str(svg, &opt).map_err(|e| format!("SVG inválido: {e}"))?;
    let size = tree.size();
    let (w, h) = ((size.width() * scale).ceil() as u32, (size.height() * scale).ceil() as u32);
    let mut pixmap = tiny_skia::Pixmap::new(w, h).ok_or("tamanho de imagem inválido")?;
    resvg::render(&tree, tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    pixmap.encode_png().map_err(|e| format!("falha ao codificar PNG: {e}"))
}

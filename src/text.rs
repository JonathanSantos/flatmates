//! Medição e quebra do título com as métricas reais da fonte Inter.
use ttf_parser::Face;

pub const INTER_BOLD: &[u8] = include_bytes!("../assets/Inter-Bold.ttf");

/// Largura do texto em pixels para um tamanho de fonte.
pub fn width(text: &str, size: f64) -> f64 {
    let Ok(face) = Face::parse(INTER_BOLD, 0) else { return text.chars().count() as f64 * size * 0.6 };
    let em = face.units_per_em() as f64;
    let units: f64 =
        text.chars().map(|c| face.glyph_index(c).and_then(|g| face.glyph_hor_advance(g)).map(|a| a as f64).unwrap_or(em * 0.6)).sum();
    units / em * size
}

fn wrap(text: &str, size: f64, max_w: f64) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        match lines.last_mut() {
            Some(line) if width(&format!("{line} {word}"), size) <= max_w => {
                line.push(' ');
                line.push_str(word);
            }
            _ => lines.push(word.to_string()),
        }
    }
    lines
}

/// Quebra o título em até 3 linhas que cabem em `max_w`, encolhendo a fonte
/// só quando a quebra não basta.
pub fn fit(text: &str, base_size: f64, max_w: f64) -> (Vec<String>, f64) {
    let mut size = base_size;
    loop {
        let lines = wrap(text, size, max_w);
        let fits = lines.len() <= 3 && lines.iter().all(|l| width(l, size) <= max_w);
        if fits || size < base_size * 0.45 {
            return (lines, size);
        }
        size *= 0.92;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wider_text_measures_wider() {
        assert!(width("Crescendo juntos", 50.0) > width("Crescendo", 50.0));
        assert!((width("ii", 50.0) - 2.0 * width("i", 50.0)).abs() < 1e-9);
    }

    #[test]
    fn short_title_stays_on_one_line_at_full_size() {
        let (lines, size) = fit("Pausa para o café", 50.0, 900.0);
        assert_eq!((lines.len(), size), (1, 50.0));
    }

    #[test]
    fn long_title_wraps_and_every_line_fits() {
        let title = "Como a nossa equipe distribuída entrega produtos melhores trabalhando de qualquer lugar";
        let (lines, size) = fit(title, 50.0, 900.0);
        assert!((2..=3).contains(&lines.len()), "{lines:?}");
        assert!(lines.iter().all(|l| width(l, size) <= 900.0));
        assert_eq!(lines.join(" "), title);
    }

    #[test]
    fn one_giant_word_shrinks_instead_of_overflowing() {
        let (lines, size) = fit("Pneumoultramicroscopicossilicovulcanoconiótico", 60.0, 900.0);
        assert_eq!(lines.len(), 1);
        assert!(size < 60.0 && width(&lines[0], size) <= 900.0);
    }
}

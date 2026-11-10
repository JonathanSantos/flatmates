//! Cores, temas e papéis de cor do Corporate Memphis.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub const fn hex(v: u32) -> Rgb {
        Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
    }

    pub fn parse(s: &str) -> Option<Rgb> {
        let h = s.strip_prefix('#')?;
        if h.len() != 6 {
            return None;
        }
        u32::from_str_radix(h, 16).ok().map(Rgb::hex)
    }

    pub fn mix(self, o: Rgb, t: f64) -> Rgb {
        let m = |a: u8, b: u8| (a as f64 + (b as f64 - a as f64) * t).round().clamp(0.0, 255.0) as u8;
        Rgb(m(self.0, o.0), m(self.1, o.1), m(self.2, o.2))
    }

    pub fn css(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }

    pub fn distance(self, o: Rgb) -> f64 {
        let d = |a: u8, b: u8| (a as f64 - b as f64).powi(2);
        (d(self.0, o.0) + d(self.1, o.1) + d(self.2, o.2)).sqrt()
    }

    /// Tom claro, usado nas faces de cima.
    pub fn light(self) -> Rgb {
        self.mix(WHITE, 0.26)
    }

    /// Tom escuro, usado nas faces laterais e nos membros do lado de lá.
    pub fn shade(self) -> Rgb {
        self.mix(INK, 0.20)
    }
}

pub const WHITE: Rgb = Rgb::hex(0xffffff);
pub const INK: Rgb = Rgb::hex(0x1e2444);
pub const SLATE: Rgb = Rgb::hex(0x4b567d);
pub const GREY: Rgb = Rgb::hex(0xc9cee0);
pub const PAPER: Rgb = Rgb::hex(0xf7f8fd);
pub const CREAM: Rgb = Rgb::hex(0xfff3e0);
pub const WOOD: Rgb = Rgb::hex(0xe0a878);
pub const LEAF: Rgb = Rgb::hex(0x19b37e);
pub const LEAF_DARK: Rgb = Rgb::hex(0x0e8f6a);
pub const LEAF_LIGHT: Rgb = Rgb::hex(0x5fd3a4);

pub const NAMED: [(&str, Rgb); 12] = [
    ("coral", Rgb::hex(0xff6b5b)),
    ("blue", Rgb::hex(0x3f6bff)),
    ("violet", Rgb::hex(0x8b5cf6)),
    ("yellow", Rgb::hex(0xffc043)),
    ("green", LEAF),
    ("pink", Rgb::hex(0xff6fa8)),
    ("ink", INK),
    ("slate", SLATE),
    ("grey", GREY),
    ("white", WHITE),
    ("cream", CREAM),
    ("wood", WOOD),
];

pub const ACCENTS: [&str; 6] = ["coral", "blue", "violet", "yellow", "green", "pink"];

pub const SKIN_FANTASY: [(&str, Rgb); 5] = [
    ("lilac", Rgb::hex(0xb18cff)),
    ("sky", Rgb::hex(0x6f9dff)),
    ("peach", Rgb::hex(0xff9a85)),
    ("teal", Rgb::hex(0x3cc9a7)),
    ("rose", Rgb::hex(0xff8fbd)),
];

pub const SKIN_REAL: [(&str, Rgb); 5] = [
    ("porcelain", Rgb::hex(0xf8cdb0)),
    ("sand", Rgb::hex(0xe5ae86)),
    ("amber", Rgb::hex(0xc08457)),
    ("umber", Rgb::hex(0x8f5a3c)),
    ("espresso", Rgb::hex(0x5e3b29)),
];

pub const HAIR_COLORS: [(&str, Rgb); 6] = [
    ("ink", INK),
    ("brown", Rgb::hex(0x5b3423)),
    ("auburn", Rgb::hex(0xc4432f)),
    ("blonde", Rgb::hex(0xf7b93e)),
    ("silver", Rgb::hex(0xdfe2ee)),
    ("plum", Rgb::hex(0x5b2a86)),
];

pub fn named(name: &str) -> Option<Rgb> {
    NAMED.iter().find(|(n, _)| *n == name).map(|(_, c)| *c)
}

fn lookup(table: &[(&str, Rgb)], name: &str) -> Option<Rgb> {
    table.iter().find(|(n, _)| *n == name).map(|(_, c)| *c)
}

/// Resolve um nome de cor de roupa ou objeto, ou um hex.
pub fn resolve(name: &str) -> Option<Rgb> {
    named(name).or_else(|| Rgb::parse(name))
}

pub fn resolve_skin(name: &str) -> Option<Rgb> {
    lookup(&SKIN_FANTASY, name).or_else(|| lookup(&SKIN_REAL, name)).or_else(|| Rgb::parse(name))
}

pub fn resolve_hair(name: &str) -> Option<Rgb> {
    lookup(&HAIR_COLORS, name).or_else(|| resolve(name))
}

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub bg: Rgb,
    pub blob: Rgb,
    pub blob2: Rgb,
    pub ground: Rgb,
    /// Fundo escuro: os neutros das roupas e dos móveis clareiam para não sumir.
    pub dark: bool,
}

pub const THEMES: [&str; 5] = ["day", "pastel", "sunset", "mint", "night"];

pub fn theme(name: &str) -> Option<Theme> {
    let t = |bg, blob, blob2, ground, dark| Theme {
        bg: Rgb::hex(bg),
        blob: Rgb::hex(blob),
        blob2: Rgb::hex(blob2),
        ground: Rgb::hex(ground),
        dark,
    };
    match name {
        "day" => Some(t(0xdce9ff, 0xbcd0ff, 0xffdcd2, 0x9db8f3, false)),
        "pastel" => Some(t(0xf3eeff, 0xf9cfdd, 0xd5e5fb, 0xd3c2f3, false)),
        "sunset" => Some(t(0xffecdc, 0xffc8aa, 0xffe2a8, 0xf3b08c, false)),
        "mint" => Some(t(0xe0f6ee, 0xb4e6d3, 0xffe4ba, 0x8cd2b7, false)),
        "night" => Some(t(0x171b3d, 0x262d66, 0x3a2c78, 0x2f387d, true)),
        _ => None,
    }
}

impl Rgb {
    pub fn luma(self) -> f64 {
        (0.299 * self.0 as f64 + 0.587 * self.1 as f64 + 0.114 * self.2 as f64) / 255.0
    }
}

/// Tema derivado de uma cor de fundo qualquer (cor de marca).
pub fn theme_from_bg(bg: Rgb, accents: &[Rgb]) -> Theme {
    let dark = bg.luma() < 0.38;
    let a = accents.first().copied().unwrap_or(Rgb::hex(0x3f6bff));
    let b = accents.get(1).copied().unwrap_or(Rgb::hex(0xff6b5b));
    if dark {
        return Theme { bg, blob: bg.mix(WHITE, 0.09), blob2: bg.mix(a, 0.28), ground: bg.mix(WHITE, 0.17), dark };
    }
    Theme { bg, blob: bg.mix(a, 0.18), blob2: bg.mix(b, 0.20), ground: bg.mix(a, 0.34).mix(INK, 0.06), dark }
}

/// O que muda o desenho de uma cena inteira: acentos, fundo escuro, sombra tonal.
#[derive(Clone, Debug)]
pub struct Style {
    pub accents: Vec<Rgb>,
    pub dark: bool,
    pub shading: bool,
}

impl Style {
    pub fn new(palette: Option<Vec<Rgb>>, dark: bool, shading: bool) -> Style {
        let accents = palette.filter(|p| !p.is_empty()).unwrap_or_else(|| ACCENTS.iter().filter_map(|n| named(n)).collect());
        Style { accents, dark, shading }
    }
    pub fn accent(&self, i: usize) -> Rgb {
        self.accents[i % self.accents.len()]
    }
    /// Metal das pernas de móveis.
    pub fn steel(&self) -> Rgb {
        if self.dark {
            SLATE.mix(WHITE, 0.22)
        } else {
            SLATE.mix(INK, 0.35)
        }
    }
    /// Cor de texto e de traços sobre o fundo.
    pub fn text(&self) -> Rgb {
        if self.dark {
            Rgb::hex(0xf2f4ff)
        } else {
            INK
        }
    }
    /// Neutros para calças.
    pub fn bottoms(&self) -> [Rgb; 4] {
        if self.dark {
            return [Rgb::hex(0x5d6ab5), Rgb::hex(0x7a86cf), Rgb::hex(0x4f5a9c), Rgb::hex(0xd9dcf0)];
        }
        [INK, SLATE.mix(INK, 0.3), INK, Rgb::hex(0x2c3b80)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_roundtrip() {
        assert_eq!(Rgb::parse("#ff6b5b").unwrap().css(), "#ff6b5b");
        assert!(Rgb::parse("ff6b5b").is_none());
    }

    #[test]
    fn every_accent_and_theme_resolves() {
        for a in ACCENTS {
            assert!(named(a).is_some(), "{a}");
        }
        for t in THEMES {
            assert!(theme(t).is_some(), "{t}");
        }
    }
}

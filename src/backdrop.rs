//! Vinheta Memphis em volta do grupo: blobs, formas geométricas e confete.
//! Tudo aqui é 2D, em pixels, e fica atrás da cena.
use crate::color::{Rgb, Style, Theme};
use crate::math::{v2, V2};
use crate::prim::{blob_path, num};
use crate::rng::Rng;
use std::f64::consts::TAU;
use std::fmt::Write;

pub struct Area {
    pub w: f64,
    pub h: f64,
    /// Caixa do grupo em pixels: [x0, y0, x1, y1].
    pub focus: [f64; 4],
    /// Onde o título termina, em pixels (0 sem título).
    pub title_bottom: f64,
}

fn blob(c: V2, rx: f64, ry: f64, rng: &mut Rng) -> String {
    let n = 9;
    let turn = rng.range(0.0, TAU);
    let pts: Vec<V2> = (0..n)
        .map(|i| {
            let a = turn + TAU * i as f64 / n as f64 + rng.range(-0.12, 0.12);
            let k = rng.range(0.88, 1.06);
            v2(c.x + rx * k * a.cos(), c.y + ry * k * a.sin())
        })
        .collect();
    blob_path(&pts)
}

pub fn blobs(area: &Area, th: &Theme, rng: &mut Rng) -> String {
    let [x0, y0, x1, y1] = area.focus;
    let c = v2((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let rx = ((x1 - x0) * 0.52 + 30.0).clamp(area.w * 0.24, area.w * 0.42);
    let ry = ((y1 - y0) * 0.54 + 24.0).clamp(area.h * 0.28, area.h * 0.43);
    let shift = v2(rx * rng.range(0.14, 0.24) * if rng.f() < 0.5 { -1.0 } else { 1.0 }, -ry * rng.range(0.10, 0.2));
    format!(
        "<path d=\"{}\" fill=\"{}\"/><path d=\"{}\" fill=\"{}\"/>",
        blob(c + shift, rx * 0.86, ry * 0.92, rng),
        th.blob2.css(),
        blob(c, rx, ry, rng),
        th.blob.css()
    )
}

fn geometric(kind: usize, c: V2, r: f64, color: Rgb, rng: &mut Rng) -> String {
    let fill = color.css();
    let (x, y) = (c.x, c.y);
    let turn = [0.0, 90.0, 180.0, 270.0][rng.below(4)];
    let body = match kind {
        0 => format!(
            "<path d=\"M{} {}h{}a{r} {r} 0 0 0 -{r} -{r}Z\" fill=\"{fill}\"/>",
            num(x - r / 2.0),
            num(y + r / 2.0),
            num(r),
            r = num(r)
        ),
        1 => format!("<path d=\"M{} {}a{r} {r} 0 0 1 {} 0Z\" fill=\"{fill}\"/>", num(x - r), num(y), num(2.0 * r), r = num(r)),
        2 => format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"none\" stroke=\"{fill}\" stroke-width=\"{}\"/>",
            num(x),
            num(y),
            num(r * 0.62),
            num(r * 0.26)
        ),
        3 => format!(
            "<path d=\"M{} {}L{} {}L{} {}Z\" fill=\"{fill}\" stroke=\"{fill}\" stroke-width=\"{}\" stroke-linejoin=\"round\"/>",
            num(x),
            num(y - r * 0.7),
            num(x + r * 0.78),
            num(y + r * 0.6),
            num(x - r * 0.78),
            num(y + r * 0.6),
            num(r * 0.16)
        ),
        4 => {
            let mut s = String::new();
            for i in 0..4 {
                for j in 0..4 {
                    let _ = write!(
                        s,
                        "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{fill}\"/>",
                        num(x + (i as f64 - 1.5) * r * 0.42),
                        num(y + (j as f64 - 1.5) * r * 0.42),
                        num(r * 0.075)
                    );
                }
            }
            s
        }
        _ => {
            let step = r * 0.42;
            format!(
                "<path d=\"M{} {}q{} {} {} 0t{s} 0t{s} 0t{s} 0\" fill=\"none\" stroke=\"{fill}\" stroke-width=\"{}\" stroke-linecap=\"round\"/>",
                num(x - step * 2.0), num(y), num(step / 2.0), num(-step * 0.9), num(step), num(r * 0.13), s = num(step)
            )
        }
    };
    format!("<g transform=\"rotate({turn} {} {})\">{body}</g>", num(x), num(y))
}

pub fn shapes(area: &Area, style: &Style, rng: &mut Rng) -> String {
    let top = (area.title_bottom / area.h + 0.14).clamp(0.20, 0.5);
    let slots = [(0.085, top), (0.915, top + 0.04), (0.07, 0.66), (0.93, 0.62)];
    let first = rng.below(6);
    let r = area.h * 0.075;
    let mut out = String::new();
    for (i, (sx, sy)) in slots.into_iter().enumerate() {
        let c = v2(area.w * (sx + rng.range(-0.02, 0.02)), area.h * (sy + rng.range(-0.04, 0.04)));
        let color = style.accent(first + i * 2);
        out.push_str(&geometric((first + i * 5) % 6, c, r * rng.range(0.85, 1.2), color, rng));
    }
    out
}

pub fn confetti(area: &Area, style: &Style, rng: &mut Rng) -> String {
    let [x0, y0, x1, y1] = area.focus;
    let pad = 26.0;
    let mut out = String::new();
    let mut placed: Vec<V2> = Vec::new();
    let mut tries = 0;
    while placed.len() < 11 && tries < 400 {
        tries += 1;
        let p = v2(rng.range(0.03, 0.97) * area.w, rng.range(0.05, 0.9) * area.h);
        let in_focus = p.x > x0 - pad && p.x < x1 + pad && p.y > y0 - pad && p.y < y1 + pad;
        let in_title = p.y < area.title_bottom + area.h * 0.05 && (p.x - area.w / 2.0).abs() < area.w * 0.44;
        if in_focus || in_title || placed.iter().any(|q| (*q - p).len() < area.h * 0.11) {
            continue;
        }
        placed.push(p);
        let fill = style.accent(rng.below(style.accents.len())).css();
        let u = area.h * 0.011;
        match rng.below(4) {
            0 => {
                let _ =
                    write!(out, "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{fill}\"/>", num(p.x), num(p.y), num(u * rng.range(0.6, 1.0)));
            }
            1 => {
                let _ = write!(
                    out,
                    "<path d=\"M{} {}h{}M{} {}v{}\" stroke=\"{fill}\" stroke-width=\"{}\" stroke-linecap=\"round\"/>",
                    num(p.x - u),
                    num(p.y),
                    num(2.0 * u),
                    num(p.x),
                    num(p.y - u),
                    num(2.0 * u),
                    num(u * 0.45)
                );
            }
            2 => {
                let _ = write!(
                    out,
                    "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"none\" stroke=\"{fill}\" stroke-width=\"{}\"/>",
                    num(p.x),
                    num(p.y),
                    num(u * 0.9),
                    num(u * 0.4)
                );
            }
            _ => {
                let _ =
                    write!(
                    out,
                    "<path d=\"M{} {}q{} {} {} 0t{s} 0\" fill=\"none\" stroke=\"{fill}\" stroke-width=\"{}\" stroke-linecap=\"round\"/>",
                    num(p.x - 2.0 * u), num(p.y), num(u), num(-u * 1.8), num(2.0 * u), num(u * 0.45), s = num(2.0 * u)
                );
            }
        }
    }
    out
}

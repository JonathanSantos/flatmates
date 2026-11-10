//! Ordem de pintura por oclusão real.
//!
//! Comparar só a profundidade do centro erra com peças grandes (tampo de
//! mesa, assento). Aqui cada par que se sobrepõe na tela é comparado pelo
//! plano da face, e o resultado vira um grafo ordenado topologicamente.
use crate::math::{V2, V3};

pub struct SortItem {
    /// Pontos representativos em espaço de vista.
    pub pts: Vec<V3>,
    /// Normal em espaço de vista, para faces planas.
    pub normal: Option<V3>,
    /// Profundidade do centro, já com o empurrão de desempate.
    pub depth: f64,
    pub bbox: [f64; 4],
    /// Contorno convexo na tela; vazio quando só a caixa é conhecida.
    pub outline: Vec<V2>,
    /// Plano (ponto, normal) em espaço de vista acima do qual a peça não existe.
    pub ceiling: Option<(V3, V3)>,
}

const EPS: f64 = 0.004;

/// De que lado do plano de `plane` a outra peça está: 1 na frente (lado da
/// câmera), -1 atrás, 0 se cruza o plano ou encosta nele.
fn side(plane: &SortItem, other: &SortItem) -> i8 {
    let Some(n) = plane.normal else { return 0 };
    let sign = if n.z >= 0.0 { 1.0 } else { -1.0 };
    let p0 = plane.pts[0];
    let (mut front, mut back) = (true, true);
    for p in &other.pts {
        let s = n.dot(*p - p0) * sign;
        if s < -EPS {
            front = false;
        }
        if s > EPS {
            back = false;
        }
    }
    match (front, back) {
        (true, false) => 1,
        (false, true) => -1,
        _ => 0,
    }
}

fn inside(poly: &[V2], p: V2) -> bool {
    let n = poly.len();
    let mut sign = 0.0;
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        let c = (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
        if c.abs() < 1e-12 {
            continue;
        }
        if sign == 0.0 {
            sign = c.signum();
        } else if c.signum() != sign {
            return false;
        }
    }
    true
}

fn distance_to(poly: &[V2], p: V2) -> f64 {
    let n = poly.len();
    (0..n)
        .map(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            let ab = b - a;
            let t = (((p.x - a.x) * ab.x + (p.y - a.y) * ab.y) / (ab.x * ab.x + ab.y * ab.y).max(1e-12)).clamp(0.0, 1.0);
            (p - (a + ab * t)).len()
        })
        .fold(f64::MAX, f64::min)
}

/// Peça que cruza o plano de uma face: de que lado ela está *onde as duas
/// se encontram na tela*. Olha só os pontos da peça que caem sobre a face
/// (ou o mais próximo dela); `Some(true)` é o lado da câmera.
fn center_side(plane: &SortItem, other: &SortItem) -> Option<bool> {
    let n = plane.normal?;
    let sign = if n.z >= 0.0 { 1.0 } else { -1.0 };
    let flat = |p: &V3| V2 { x: p.x, y: p.y };
    let mut near: Vec<V3> = match plane.outline.len() {
        0..=2 => other.pts.clone(),
        _ => other.pts.iter().filter(|p| inside(&plane.outline, flat(p))).copied().collect(),
    };
    if near.is_empty() {
        let closest =
            other.pts.iter().min_by(|a, b| distance_to(&plane.outline, flat(a)).total_cmp(&distance_to(&plane.outline, flat(b))))?;
        near.push(*closest);
    }
    let c = near.iter().fold(V3::default(), |a, p| a + *p) * (1.0 / near.len() as f64);
    let s = n.dot(c - plane.pts[0]) * sign;
    if s.abs() < EPS {
        return None;
    }
    Some(s > 0.0)
}

fn overlap(a: &[f64; 4], b: &[f64; 4]) -> bool {
    a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3]
}

/// Algum lado de `a` separa os dois polígonos convexos?
fn separated_by(a: &[V2], b: &[V2]) -> bool {
    let n = a.len();
    (0..n).any(|i| {
        let (p, q) = (a[i], a[(i + 1) % n]);
        let (nx, ny) = (q.y - p.y, p.x - q.x);
        let side = |v: &V2| (v.x - p.x) * nx + (v.y - p.y) * ny;
        let inner = a.iter().map(side).fold(0.0_f64, |m, s| if s.abs() > m.abs() { s } else { m });
        // `b` inteiro do lado oposto ao interior de `a`.
        b.iter().all(|v| side(v) * inner <= 1e-9)
    })
}

/// Sobreposição real na tela: caixas e, quando há contorno, eixo separador.
fn touching(a: &SortItem, b: &SortItem) -> bool {
    if !overlap(&a.bbox, &b.bbox) {
        return false;
    }
    if a.outline.len() < 3 || b.outline.len() < 3 {
        return true;
    }
    !separated_by(&a.outline, &b.outline) && !separated_by(&b.outline, &a.outline)
}

/// `Some(true)` quando `a` precisa ser pintada antes de `b`.
fn before(a: &SortItem, b: &SortItem) -> Option<bool> {
    if !touching(a, b) {
        return None;
    }
    // Quem está inteiro acima do teto de uma peça nunca é coberto por ela.
    for (low, high, low_first) in [(a, b, true), (b, a, false)] {
        if let Some((p0, n)) = low.ceiling {
            if n.z > 0.0 && high.pts.iter().all(|p| n.dot(*p - p0) >= -EPS) {
                return Some(low_first);
            }
        }
    }
    let (sa, sb) = (side(a, b), side(b, a));
    match (a.normal.is_some(), b.normal.is_some()) {
        (true, true) => match (sa, sb) {
            // Vizinhas de um sólido convexo ou canto côncavo: não se cobrem.
            (1, 1) | (-1, -1) => return None,
            (1, _) | (0, -1) => return Some(true),
            (-1, _) | (0, 1) => return Some(false),
            _ => {}
        },
        (true, false) if sa != 0 => return Some(sa == 1),
        (false, true) if sb != 0 => return Some(sb == -1),
        // Peça que cruza o plano de uma face: vale o lado onde está o centro dela.
        (true, false) => {
            if let Some(front) = center_side(a, b) {
                return Some(front);
            }
        }
        (false, true) => {
            if let Some(front) = center_side(b, a) {
                return Some(!front);
            }
        }
        _ => {}
    }
    let d = a.depth - b.depth;
    if d.abs() < 1e-6 {
        return None;
    }
    Some(d < 0.0)
}

/// Entre as peças ainda não pintadas, as que estão de fato num ciclo: tira
/// repetidamente quem não tem ninguém antes ou ninguém depois.
fn cyclic_core(after: &[Vec<usize>], done: &[bool]) -> Vec<usize> {
    let n = done.len();
    let mut alive: Vec<bool> = done.iter().map(|d| !d).collect();
    loop {
        let mut incoming = vec![0usize; n];
        for i in (0..n).filter(|&i| alive[i]) {
            for &j in after[i].iter().filter(|&&j| alive[j]) {
                incoming[j] += 1;
            }
        }
        let drop: Vec<usize> = (0..n).filter(|&i| alive[i] && (incoming[i] == 0 || !after[i].iter().any(|&j| alive[j]))).collect();
        if drop.is_empty() {
            break;
        }
        drop.into_iter().for_each(|i| alive[i] = false);
    }
    (0..n).filter(|&i| alive[i]).collect()
}

/// Índices na ordem de pintura (do fundo para a frente), e as peças que
/// estavam presas num ciclo quando a ordem precisou ser forçada.
pub fn order(items: &[SortItem]) -> (Vec<usize>, Vec<usize>) {
    let n = items.len();
    let mut after: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut pending = vec![0usize; n];
    for i in 0..n {
        for j in i + 1..n {
            match before(&items[i], &items[j]) {
                Some(true) => {
                    after[i].push(j);
                    pending[j] += 1;
                }
                Some(false) => {
                    after[j].push(i);
                    pending[i] += 1;
                }
                None => {}
            }
        }
    }
    let mut done = vec![false; n];
    let mut out = Vec::with_capacity(n);
    let mut stuck: Vec<usize> = Vec::new();
    for _ in 0..n {
        let farthest = |free_only: bool| {
            (0..n)
                .filter(|&i| !done[i] && (!free_only || pending[i] == 0))
                .min_by(|&a, &b| items[a].depth.total_cmp(&items[b].depth).then(a.cmp(&b)))
        };
        let pick = match farthest(true) {
            Some(p) => p,
            None => {
                // Ciclo: guarda quem está preso e quebra pelo mais distante.
                if stuck.is_empty() {
                    stuck = cyclic_core(&after, &done);
                }
                match farthest(false) {
                    Some(p) => p,
                    None => break,
                }
            }
        };
        done[pick] = true;
        out.push(pick);
        for &j in &after[pick] {
            pending[j] = pending[j].saturating_sub(1);
        }
    }
    (out, stuck)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::v3;

    fn blob(z: f64) -> SortItem {
        SortItem { pts: vec![v3(0.0, 0.0, z)], normal: None, depth: z, bbox: [-1.0, -1.0, 1.0, 1.0], outline: vec![], ceiling: None }
    }

    #[test]
    fn farther_blob_is_painted_first() {
        let items = [blob(2.0), blob(-1.0), blob(0.5)];
        assert_eq!(order(&items).0, vec![1, 2, 0]);
    }

    #[test]
    fn plane_wins_over_centroid_depth() {
        // Tampo grande olhando para cima e para a câmera; o objeto está em
        // cima dele, mas mais ao fundo que o centro do tampo.
        let n = v3(0.0, 0.94, 0.34);
        let top = SortItem {
            pts: vec![v3(-1.0, 0.0, 0.0), v3(1.0, 0.0, 0.0)],
            normal: Some(n),
            depth: 0.0,
            bbox: [-1.0, -1.0, 1.0, 1.0],
            outline: vec![],
            ceiling: None,
        };
        let on_top = SortItem {
            pts: vec![v3(0.0, 0.0, 0.0) + n * 0.1 + v3(0.0, 0.34, -0.94) * 0.5],
            normal: None,
            depth: -0.4,
            bbox: [-0.1, -0.1, 0.1, 0.1],
            outline: vec![],
            ceiling: None,
        };
        assert_eq!(order(&[on_top, top]).0, vec![1, 0]);
    }

    #[test]
    fn crossing_faces_never_drop_a_piece() {
        // Três faces que se cobrem em roda: A antes de B, B antes de C, C antes de A.
        let face = |p0: V3, n: V3, others: [V3; 2]| SortItem {
            pts: vec![p0, others[0], others[1]],
            normal: Some(n),
            depth: 0.0,
            bbox: [-1.0, -1.0, 1.0, 1.0],
            outline: vec![],
            ceiling: None,
        };
        let up = v3(0.0, 0.0, 1.0);
        let a = face(v3(0.0, 0.0, 0.0), up, [v3(1.0, 0.0, 0.0), v3(0.0, 1.0, 0.0)]);
        let mut b = face(v3(0.0, 0.0, 1.0), up, [v3(1.0, 0.0, 1.0), v3(0.0, 1.0, 1.0)]);
        let mut c = face(v3(0.0, 0.0, 2.0), up, [v3(1.0, 0.0, 2.0), v3(0.0, 1.0, 2.0)]);
        // Sem ciclo: ordem limpa.
        let (o, stuck) = order(&[a, b, c]);
        assert_eq!((o, stuck.len()), (vec![0, 1, 2], 0));
        // Com faces inclinadas que se cruzam, cai no desempate e não trava.
        b = face(v3(0.0, 0.0, 0.0), v3(0.6, 0.0, 0.8), [v3(1.0, 0.0, -0.75), v3(-1.0, 0.0, 0.75)]);
        c = face(v3(0.0, 0.0, 0.0), v3(-0.6, 0.0, 0.8), [v3(1.0, 0.0, 0.75), v3(-1.0, 0.0, -0.75)]);
        let a2 = face(v3(0.0, 0.0, 0.0), up, [v3(1.0, 0.0, 0.0), v3(-1.0, 0.0, 0.0)]);
        assert_eq!(order(&[a2, b, c]).0.len(), 3);
    }

    #[test]
    fn outlines_that_only_share_a_box_do_not_touch() {
        use crate::math::v2;
        // Dois triângulos em cantos opostos da mesma caixa.
        let tri = |pts: [V2; 3], z: f64| SortItem {
            pts: vec![v3(0.0, 0.0, z)],
            normal: None,
            depth: z,
            bbox: [0.0, 0.0, 1.0, 1.0],
            outline: pts.to_vec(),
            ceiling: None,
        };
        let a = tri([v2(0.0, 0.0), v2(0.4, 0.0), v2(0.0, 0.4)], 1.0);
        let b = tri([v2(1.0, 1.0), v2(0.6, 1.0), v2(1.0, 0.6)], 0.0);
        assert!(before(&a, &b).is_none());
        let c = tri([v2(0.0, 0.0), v2(0.9, 0.0), v2(0.0, 0.9)], 0.0);
        let d = tri([v2(0.2, 0.2), v2(1.0, 0.3), v2(0.3, 1.0)], 1.0);
        assert_eq!(before(&c, &d), Some(true));
    }

    #[test]
    fn disjoint_boxes_do_not_constrain_each_other() {
        let mut a = blob(1.0);
        a.bbox = [0.0, 0.0, 1.0, 1.0];
        let mut b = blob(0.0);
        b.bbox = [2.0, 2.0, 3.0, 3.0];
        assert!(before(&a, &b).is_none());
    }
}

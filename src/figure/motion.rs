//! Movimentos cíclicos: uma lista de poses-chave interpolada no tempo.
//! Como o rig resolve qualquer pose, animar é só mover as metas.
use super::pose::{pose, FootGoal, HandGoal, Pose};
use crate::math::V3;

pub const MOTIONS: [&str; 5] = ["walk", "run", "wave", "jump", "type"];

fn mix(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

fn mix_v(a: V3, b: V3, t: f64) -> V3 {
    a + (b - a) * t
}

/// Mistura duas poses. O que não é contínuo (sentado, formato da mão) vem de `a`.
pub fn lerp(a: &Pose, b: &Pose, t: f64) -> Pose {
    let foot = |i: usize| FootGoal {
        out: mix(a.feet[i].out, b.feet[i].out, t),
        fwd: mix(a.feet[i].fwd, b.feet[i].fwd, t),
        lift: mix(a.feet[i].lift, b.feet[i].lift, t),
        pitch: mix(a.feet[i].pitch, b.feet[i].pitch, t),
    };
    let hand = |i: usize| HandGoal { at: mix_v(a.hands[i].at, b.hands[i].at, t), pole: mix_v(a.hands[i].pole, b.hands[i].pole, t) };
    Pose {
        crouch: mix(a.crouch, b.crouch, t),
        lean: mix(a.lean, b.lean, t),
        tilt: mix(a.tilt, b.tilt, t),
        curve: mix(a.curve, b.curve, t),
        hip: mix(a.hip, b.hip, t),
        twist: mix(a.twist, b.twist, t),
        head_yaw: mix(a.head_yaw, b.head_yaw, t),
        head_pitch: mix(a.head_pitch, b.head_pitch, t),
        air: mix(a.air, b.air, t),
        knees: [mix_v(a.knees[0], b.knees[0], t), mix_v(a.knees[1], b.knees[1], t)],
        feet: [foot(0), foot(1)],
        hands: [hand(0), hand(1)],
        ..*a
    }
}

/// Troca esquerda e direita.
pub fn mirror(p: &Pose) -> Pose {
    Pose {
        tilt: -p.tilt,
        curve: -p.curve,
        hip: -p.hip,
        twist: -p.twist,
        head_yaw: -p.head_yaw,
        knees: [p.knees[1], p.knees[0]],
        grips: [p.grips[1], p.grips[0]],
        feet: [p.feet[1], p.feet[0]],
        hands: [p.hands[1], p.hands[0]],
        ..*p
    }
}

fn keys(name: &str) -> Option<Vec<Pose>> {
    let get = |n: &str| pose(n).expect("pose nativa");
    Some(match name {
        "walk" => {
            let contact = get("walking");
            // Passagem: o pé de trás cruza no ar ao lado do pé de apoio.
            let mut pass = lerp(&contact, &mirror(&contact), 0.5);
            pass.crouch = 0.01;
            pass.feet[0] = FootGoal { out: 0.0, fwd: 0.0, lift: 0.0, pitch: 0.0 };
            pass.feet[1] = FootGoal { out: 0.0, fwd: 0.04, lift: 0.11, pitch: 24.0 };
            vec![contact, pass, mirror(&contact), mirror(&pass)]
        }
        "run" => {
            let stride = get("running");
            let mut gather = lerp(&stride, &mirror(&stride), 0.5);
            gather.air = 0.0;
            gather.crouch = 0.06;
            gather.feet[0] = FootGoal { out: 0.0, fwd: 0.02, lift: 0.0, pitch: 6.0 };
            gather.feet[1] = FootGoal { out: 0.0, fwd: -0.10, lift: 0.34, pitch: 70.0 };
            vec![stride, gather, mirror(&stride), mirror(&gather)]
        }
        "wave" => {
            let a = get("waving");
            let mut b = a;
            b.hands[1].at.x = 0.22;
            b.hands[1].at.y = 0.66;
            b.curve = a.curve * 0.6;
            vec![a, b]
        }
        "jump" => {
            let stand = get("standing");
            let mut load = get("crouching");
            load.crouch = 0.26;
            load.lean = 16.0;
            load.hands = stand.hands;
            load.hands[0].at.z = -0.3;
            load.hands[1].at.z = -0.3;
            vec![stand, load, get("jumping"), load]
        }
        "type" => {
            let a = get("typing");
            let (mut b, mut c) = (a, a);
            b.hands[0].at.y += 0.035;
            c.hands[1].at.y += 0.035;
            c.head_pitch += 3.0;
            vec![a, b, a, c]
        }
        _ => return None,
    })
}

/// Pose no instante `t` do ciclo (0 a 1, repete).
pub fn sample(name: &str, t: f64) -> Option<Pose> {
    let ks = keys(name)?;
    let n = ks.len();
    let x = t.rem_euclid(1.0) * n as f64;
    let i = (x.floor() as usize) % n;
    let f = x - x.floor();
    // Suaviza a passagem entre poses-chave.
    let ease = f * f * (3.0 - 2.0 * f);
    Some(lerp(&ks[i], &ks[(i + 1) % n], ease))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_closes_and_hits_the_keys() {
        for m in MOTIONS {
            let (a, b) = (sample(m, 0.0).unwrap(), sample(m, 1.0).unwrap());
            assert!((a.feet[0].fwd - b.feet[0].fwd).abs() < 1e-9, "{m}");
            assert!((a.hands[1].at - b.hands[1].at).len() < 1e-9, "{m}");
        }
    }

    #[test]
    fn walk_swaps_the_leading_foot_half_way() {
        let (a, b) = (sample("walk", 0.0).unwrap(), sample("walk", 0.5).unwrap());
        assert!(a.feet[0].fwd > 0.2 && b.feet[1].fwd > 0.2);
        assert!((a.feet[0].fwd - b.feet[1].fwd).abs() < 1e-9);
    }

    #[test]
    fn mirror_twice_is_identity() {
        let p = pose("waving").unwrap();
        let q = mirror(&mirror(&p));
        assert_eq!(p.hands[1].at, q.hands[1].at);
        assert_eq!(p.curve, q.curve);
    }
}

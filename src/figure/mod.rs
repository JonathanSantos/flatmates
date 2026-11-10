//! Rig do personagem: um esqueleto 3D em metros. Corpo, roupa, cabelo e
//! acessórios são presos às juntas, então acompanham qualquer pose e câmera.
pub mod draw;
pub mod head;
pub mod motion;
pub mod pose;

use crate::math::Det;
use crate::math::{two_bone, v3, UP, V3};
use pose::{Grip, Pose};

pub const BUILDS: [&str; 4] = ["regular", "slim", "broad", "curvy"];

/// Medidas do corpo, já na escala do personagem.
#[derive(Clone, Copy, Debug)]
pub struct Dims {
    pub s: f64,
    pub ankle_h: f64,
    pub shin: f64,
    pub thigh: f64,
    pub hip_half: f64,
    pub torso: f64,
    pub shoulder_half: f64,
    pub neck: f64,
    pub head_r: f64,
    pub upper_arm: f64,
    pub forearm: f64,
    pub r_thigh: f64,
    pub r_knee: f64,
    pub r_ankle: f64,
    pub r_arm: f64,
    pub r_elbow: f64,
    pub r_wrist: f64,
    pub r_chest: f64,
    /// Meia-distância entre as duas esferas da cintura.
    pub waist_half: f64,
    /// Raio das esferas da cintura.
    pub r_mid: f64,
    pub r_hip: f64,
    /// Profundidade do peito, vista de perfil.
    pub r_bust: f64,
    pub r_belly: f64,
}

impl Dims {
    /// `height` em metros; `build` é uma das formas de `BUILDS`.
    pub fn new(height: f64, build: &str) -> Dims {
        let s = height / 1.72;
        // (grossura dos membros, ombros, quadril, cintura, peito, barriga)
        let (width, shoulders, hips, waist, bust, belly) = match build {
            "slim" => (0.86, 0.93, 0.90, 0.84, 0.88, 0.80),
            "broad" => (1.14, 1.17, 1.06, 1.24, 1.16, 1.32),
            "curvy" => (1.05, 0.95, 1.15, 0.88, 1.12, 1.00),
            _ => (1.0, 1.0, 1.0, 1.0, 1.0, 1.0),
        };
        let w = s * width;
        Dims {
            s,
            ankle_h: 0.09 * s,
            shin: 0.41 * s,
            thigh: 0.42 * s,
            hip_half: 0.085 * s * hips,
            torso: 0.50 * s,
            shoulder_half: 0.165 * s * shoulders,
            neck: 0.088 * s,
            head_r: 0.105 * s.sqrt(),
            upper_arm: 0.29 * s,
            forearm: 0.26 * s,
            r_thigh: 0.080 * w * hips.sqrt(),
            r_knee: 0.057 * w,
            r_ankle: 0.040 * w,
            r_arm: 0.050 * w,
            r_elbow: 0.042 * w,
            r_wrist: 0.034 * w,
            r_chest: 0.074 * w,
            waist_half: 0.068 * s * waist,
            r_mid: 0.080 * s * waist,
            r_hip: 0.092 * s * hips,
            r_bust: 0.104 * s * bust,
            r_belly: 0.100 * s * belly,
        }
    }

    pub fn hip_height(&self) -> f64 {
        self.ankle_h + self.shin + self.thigh - 0.012 * self.s
    }
}

/// Onde o personagem está no mundo.
#[derive(Clone, Copy, Debug)]
pub struct Placement {
    /// Ponto do chão embaixo do quadril.
    pub pos: V3,
    /// Para onde olha, em radianos: 0 é para a câmera frontal, 90° é para a direita.
    pub facing: f64,
    /// Altura do assento acima de `pos`, quando sentado.
    pub seat: Option<f64>,
}

/// Juntas resolvidas, em coordenadas do mundo. Índice 0 é o lado esquerdo.
#[derive(Clone, Debug)]
pub struct Rig {
    pub d: Dims,
    pub pelvis: V3,
    /// Meio da coluna, onde ela muda de direção.
    pub waist: V3,
    pub chest: V3,
    pub head: V3,
    pub hip: [V3; 2],
    pub knee: [V3; 2],
    pub ankle: [V3; 2],
    pub foot_fwd: [V3; 2],
    pub foot_up: [V3; 2],
    pub shoulder: [V3; 2],
    pub elbow: [V3; 2],
    pub wrist: [V3; 2],
    pub fwd: V3,
    /// Aponta para o lado esquerdo do personagem.
    pub side: V3,
    pub spine: V3,
    pub head_fwd: V3,
    pub head_up: V3,
    pub grips: [Grip; 2],
    /// Altura do tampo do assento no mundo, quando sentado.
    pub seat_top: Option<f64>,
    /// Algum alvo de mão ou pé ficou fora do alcance.
    pub strained: bool,
}

pub const SIGN: [f64; 2] = [1.0, -1.0];

/// Pontos da sola no referencial do pé: (para a frente, para cima), em
/// metros a partir do tornozelo.
pub const SOLE_BACK: (f64, f64) = (-0.058, -0.081);
pub const SOLE_FRONT: (f64, f64) = (0.172, -0.081);
pub const SOLE_R: f64 = 0.009;

/// Resolve a pose. `reach` substitui a meta de uma mão por um ponto do mundo.
pub fn solve(d: &Dims, pose: &Pose, place: &Placement, reach: [Option<V3>; 2]) -> Rig {
    let s = d.s;
    let to_local = |w: V3| (w - place.pos).rot_y(-place.facing);
    let seat = place.seat.or(if pose.floor { Some(0.0) } else { None });
    let seated = seat.is_some();
    let lift_all = if seated { 0.0 } else { pose.air * s };
    let pelvis_y = match seat {
        // A coxa (com a folga da calça) apoia em cima do assento, sem afundar nele.
        Some(h) => h + d.r_thigh + 0.012 * s,
        // O quadril inclinado sobe um lado: desce o conjunto para a perna desse lado alcançar o chão.
        None => d.hip_height() - pose.crouch * s + lift_all - d.hip_half * pose.hip.to_radians().dsin().abs(),
    };
    let pelvis = v3(0.0, pelvis_y, 0.0);
    // Coluna em dois trechos: a curva lateral e parte da inclinação mudam no meio.
    let (lean, tilt, curve) = (pose.lean.to_radians(), pose.tilt.to_radians(), pose.curve.to_radians());
    let lower = UP.rot_x(lean * 0.75).rot_z(tilt - curve);
    let spine = UP.rot_x(lean * 1.2).rot_z(tilt + curve);
    let waist = pelvis + lower * (d.torso * 0.45);
    let chest = waist + spine * (d.torso * 0.55);
    let hip_axis = v3(1.0, 0.0, 0.0).rot_z(-pose.hip.to_radians());
    let shoulder_axis = v3(1.0, 0.0, 0.0).rot_y(pose.twist.to_radians());
    let mut strained = false;

    let mut hip = [V3::default(); 2];
    let mut knee = hip;
    let mut ankle = hip;
    let mut foot_fwd = hip;
    let mut foot_up = hip;
    let mut shoulder = hip;
    let mut elbow = hip;
    let mut wrist = hip;
    for i in 0..2 {
        let sign = SIGN[i];
        let goal = pose.feet[i];
        hip[i] = pelvis + hip_axis * (sign * d.hip_half);
        let flat = v3(0.0, 0.0, 1.0).rot_y(sign * 0.14);
        let p = goal.pitch.to_radians();
        let f = flat * p.dcos() - UP * p.dsin();
        let u = UP * p.dcos() + flat * p.dsin();
        let lowest = [SOLE_BACK, SOLE_FRONT].iter().map(|&(z, y)| (f * z + u * y).y * s - SOLE_R * s).fold(f64::MAX, f64::min);
        // Assento alto: os pés ficam pendurados na altura em que a canela
        // desce reta, em vez de puxar a coxa para baixo até o chão.
        // O joelho nunca fica abaixo do assento: a coxa não pode afundar na
        // borda dele. Quem é baixo para o assento fica com os pés no ar.
        let seat_top = pelvis_y - d.r_thigh - 0.012 * s;
        let excess = pelvis_y - 0.07 * s - d.shin - d.ankle_h;
        let dangle = if !seated || pose.floor {
            0.0
        } else if excess > 0.1 * s {
            excess
        } else {
            // Só para diferenças pequenas (alguém baixo numa cadeira). Num
            // assento bem mais alto sem ser banco, como a bola, a coxa desce
            // em rampa e os pés ficam no chão.
            let lift = seat_top + d.r_knee - 0.02 * s - d.shin - d.ankle_h;
            if (0.0..=0.06 * s).contains(&lift) {
                lift
            } else {
                0.0
            }
        };
        let target = v3(sign * (d.hip_half + goal.out * s), goal.lift * s + lift_all + dangle - lowest, goal.fwd * s);
        let knee_pole = pose.knees[i];
        let leg = two_bone(hip[i], target, d.thigh, d.shin, v3(sign * knee_pole.x, knee_pole.y, knee_pole.z));
        strained |= leg.strained;
        knee[i] = leg.mid;
        ankle[i] = leg.end;
        foot_fwd[i] = f;
        foot_up[i] = u;

        shoulder[i] = chest + shoulder_axis * (sign * d.shoulder_half) - spine * (0.045 * s);
        let goal = pose.hands[i];
        let arm_len = d.upper_arm + d.forearm;
        let target = match reach[i] {
            Some(w) => to_local(w),
            None => shoulder[i] + v3(sign * goal.at.x, goal.at.y, goal.at.z) * arm_len,
        };
        let pole = v3(sign * goal.pole.x, goal.pole.y, goal.pole.z);
        let arm = two_bone(shoulder[i], target, d.upper_arm, d.forearm, pole);
        strained |= arm.strained;
        elbow[i] = arm.mid;
        wrist[i] = arm.end;
    }

    let upright = 1.6 * lean.dcos() * lean.dcos();
    let head_axis = (spine + UP * upright).norm();
    let head = chest + head_axis * (d.neck + d.head_r);
    let (hy, hp) = (pose.head_yaw.to_radians(), pose.head_pitch.to_radians());
    let head_fwd = v3(0.0, 0.0, 1.0).rot_x(hp).rot_y(hy);
    let head_up = UP.rot_x(hp).rot_y(hy);

    let w = |p: V3| place.pos + p.rot_y(place.facing);
    let dir = |p: V3| p.rot_y(place.facing);
    Rig {
        d: *d,
        pelvis: w(pelvis),
        waist: w(waist),
        chest: w(chest),
        head: w(head),
        hip: hip.map(w),
        knee: knee.map(w),
        ankle: ankle.map(w),
        foot_fwd: foot_fwd.map(dir),
        foot_up: foot_up.map(dir),
        shoulder: shoulder.map(w),
        elbow: elbow.map(w),
        wrist: wrist.map(w),
        fwd: dir(v3(0.0, 0.0, 1.0)),
        side: dir(v3(1.0, 0.0, 0.0)),
        spine: dir(spine),
        head_fwd: dir(head_fwd),
        head_up: dir(head_up),
        grips: pose.grips,
        seat_top: seat.map(|h| place.pos.y + h),
        strained,
    }
}

impl Rig {
    /// Centro da mão, um pouco além do pulso.
    pub fn hand(&self, i: usize) -> V3 {
        let dir = (self.wrist[i] - self.elbow[i]).norm();
        self.wrist[i] + dir * (0.06 * self.d.s)
    }

    /// Ponto do pé no referencial (frente, cima) a partir do tornozelo.
    pub fn foot_point(&self, i: usize, fwd: f64, up: f64) -> V3 {
        self.ankle[i] + (self.foot_fwd[i] * fwd + self.foot_up[i] * up) * self.d.s
    }

    /// Como `foot_point`, com um deslocamento lateral (largura do sapato).
    pub fn foot_corner(&self, i: usize, fwd: f64, up: f64, side: f64) -> V3 {
        let across = self.foot_up[i].cross(self.foot_fwd[i]);
        self.foot_point(i, fwd, up) + across * (side * self.d.s)
    }

    /// Ossos para o desenho de depuração.
    pub fn bones(&self) -> Vec<(V3, V3)> {
        let mut b = vec![(self.pelvis, self.waist), (self.waist, self.chest), (self.chest, self.head)];
        for i in 0..2 {
            b.push((self.pelvis, self.hip[i]));
            b.push((self.hip[i], self.knee[i]));
            b.push((self.knee[i], self.ankle[i]));
            b.push((self.ankle[i], self.foot_point(i, 0.16, -0.07)));
            b.push((self.chest, self.shoulder[i]));
            b.push((self.shoulder[i], self.elbow[i]));
            b.push((self.elbow[i], self.wrist[i]));
        }
        b
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pose::{pose, POSES};

    fn rig(name: &str, seat: Option<f64>) -> Rig {
        let d = Dims::new(1.72, "regular");
        let place = Placement { pos: v3(0.3, 0.0, -0.2), facing: 1.1, seat };
        solve(&d, &pose(name).unwrap(), &place, [None, None])
    }

    fn sole_bottom(r: &Rig, i: usize) -> f64 {
        [SOLE_BACK, SOLE_FRONT].iter().map(|&(z, y)| r.foot_point(i, z, y).y - SOLE_R * r.d.s).fold(f64::MAX, f64::min)
    }

    #[test]
    fn every_pose_keeps_bone_lengths() {
        for name in POSES {
            let p = pose(name).unwrap();
            let r = rig(name, if p.seated && !p.floor { Some(0.46) } else { None });
            for i in 0..2 {
                assert!(((r.knee[i] - r.hip[i]).len() - r.d.thigh).abs() < 1e-6, "{name}");
                assert!(((r.ankle[i] - r.knee[i]).len() - r.d.shin).abs() < 1e-6, "{name}");
                assert!(((r.elbow[i] - r.shoulder[i]).len() - r.d.upper_arm).abs() < 1e-6, "{name}");
                assert!(((r.wrist[i] - r.elbow[i]).len() - r.d.forearm).abs() < 1e-6, "{name}");
            }
        }
    }

    #[test]
    fn grounded_poses_touch_the_floor_without_sinking() {
        for name in [
            "standing",
            "walking",
            "waving",
            "presenting",
            "reaching",
            "sitting",
            "typing",
            "leaning",
            "crouching",
            "floor-sitting",
            "pointing",
        ] {
            let p = pose(name).unwrap();
            let r = rig(name, if p.seated && !p.floor { Some(0.46) } else { None });
            assert!(!r.strained, "{name} fora do alcance");
            for i in 0..2 {
                if p.feet[i].lift > 0.0 {
                    continue;
                }
                assert!(sole_bottom(&r, i).abs() < 1e-6, "{name} pé {i}: {}", sole_bottom(&r, i));
            }
        }
    }

    #[test]
    fn builds_change_the_body_where_they_should() {
        let d = |b: &str| Dims::new(1.72, b);
        let (regular, slim, broad, curvy) = (d("regular"), d("slim"), d("broad"), d("curvy"));
        assert!(slim.shoulder_half < regular.shoulder_half && regular.shoulder_half < broad.shoulder_half);
        assert!(slim.r_thigh < regular.r_thigh && regular.r_thigh < broad.r_thigh);
        // Curvy: quadril mais largo e cintura mais fina que o corpo regular.
        assert!(curvy.hip_half + curvy.r_hip > regular.hip_half + regular.r_hip);
        assert!(curvy.waist_half + curvy.r_mid < regular.waist_half + regular.r_mid);
        // Broad: barriga mais funda.
        assert!(broad.r_belly > regular.r_belly * 1.2);
        // A altura dos ossos não muda com o biotipo.
        assert_eq!(slim.thigh, broad.thigh);
    }

    #[test]
    fn jumping_leaves_the_floor() {
        let r = rig("jumping", None);
        assert!(sole_bottom(&r, 0) > 0.2 && sole_bottom(&r, 1) > 0.2);
    }

    #[test]
    fn knees_bend_forward() {
        for name in ["walking", "running", "sitting"] {
            let r = rig(name, if name == "sitting" { Some(0.46) } else { None });
            for i in 0..2 {
                let mid = r.hip[i].lerp(r.ankle[i], 0.5);
                assert!((r.knee[i] - mid).dot(r.fwd) > -1e-6, "{name} joelho {i} dobrou para trás");
            }
        }
    }
}

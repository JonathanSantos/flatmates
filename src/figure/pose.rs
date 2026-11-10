//! Poses: metas para pés e mãos. O rig resolve joelhos e cotovelos.
use crate::math::{v3, V3};

/// Onde o pé deve ficar, em metros (para um corpo de 1,72 m).
#[derive(Clone, Copy, Debug)]
pub struct FootGoal {
    /// Afastamento lateral a partir do quadril (positivo abre a perna).
    pub out: f64,
    pub fwd: f64,
    /// Altura da sola acima do chão.
    pub lift: f64,
    /// Calcanhar levantado, em graus.
    pub pitch: f64,
}

/// Onde o pulso deve ficar, em comprimentos de braço a partir do ombro:
/// (para fora, para cima, para a frente). `pole` é para onde o cotovelo dobra.
#[derive(Clone, Copy, Debug)]
pub struct HandGoal {
    pub at: V3,
    pub pole: V3,
}

/// Formato da mão.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Grip {
    /// Luva relaxada.
    Relaxed,
    /// Dedos abertos: acenar, comemorar.
    Open,
    /// Indicador esticado.
    Point,
    /// Mão fechada: segurar, correr.
    Fist,
}

#[derive(Clone, Copy, Debug)]
pub struct Pose {
    /// Quanto o quadril desce (metros).
    pub crouch: f64,
    /// Inclinação do tronco para a frente (graus).
    pub lean: f64,
    /// Inclinação do tronco para o lado esquerdo do personagem (graus).
    pub tilt: f64,
    /// Curva da coluna para o lado: a parte de cima vai para um lado e a
    /// de baixo para o outro (graus). É a linha de gesto da figura.
    pub curve: f64,
    /// Quadril inclinado: positivo sobe o lado esquerdo (graus).
    pub hip: f64,
    /// Giro dos ombros em relação ao quadril (graus).
    pub twist: f64,
    pub head_yaw: f64,
    pub head_pitch: f64,
    /// Altura do salto (metros).
    pub air: f64,
    pub seated: bool,
    /// Pose de chão: o quadril apoia no piso, sem precisar de assento.
    pub floor: bool,
    /// Para onde cada joelho dobra.
    pub knees: [V3; 2],
    pub grips: [Grip; 2],
    /// Índice 0 é o lado esquerdo do personagem, 1 é o direito.
    pub feet: [FootGoal; 2],
    pub hands: [HandGoal; 2],
}

const fn foot(out: f64, fwd: f64, lift: f64, pitch: f64) -> FootGoal {
    FootGoal { out, fwd, lift, pitch }
}

const fn hand(out: f64, up: f64, fwd: f64, pole: V3) -> HandGoal {
    HandGoal { at: v3(out, up, fwd), pole }
}

const KNEE: V3 = v3(0.10, 0.12, 1.0);
const BACK: V3 = v3(0.25, 0.0, -1.0);
const DOWN_OUT: V3 = v3(0.6, -1.0, -0.2);
const REST: HandGoal = hand(0.07, -0.985, 0.04, BACK);

const BASE: Pose = Pose {
    crouch: 0.0,
    lean: 0.0,
    tilt: 0.0,
    curve: 0.0,
    hip: 0.0,
    twist: 0.0,
    head_yaw: 0.0,
    head_pitch: 0.0,
    air: 0.0,
    seated: false,
    floor: false,
    knees: [KNEE, KNEE],
    grips: [Grip::Relaxed, Grip::Relaxed],
    feet: [foot(0.015, 0.035, 0.0, 0.0), foot(0.015, -0.035, 0.0, 0.0)],
    hands: [REST, REST],
};

pub const POSES: [&str; 20] = [
    "standing",
    "walking",
    "running",
    "waving",
    "celebrating",
    "presenting",
    "thinking",
    "holding",
    "reaching",
    "carrying",
    "jumping",
    "sitting",
    "typing",
    "crouching",
    "leaning",
    "desk-typing",
    "hugging",
    "floor-sitting",
    "lying",
    "pointing",
];

pub fn pose(name: &str) -> Option<Pose> {
    let p = match name {
        // Peso numa perna e tronco levemente inclinado: evita a figura a prumo.
        "standing" => Pose {
            tilt: 1.0,
            curve: 3.5,
            hip: 4.0,
            lean: 1.5,
            head_yaw: 6.0,
            feet: [foot(0.035, 0.06, 0.0, 0.0), foot(0.0, -0.05, 0.0, 0.0)],
            hands: [REST, hand(0.06, -0.975, 0.09, BACK)],
            ..BASE
        },
        "walking" => Pose {
            crouch: 0.05,
            lean: 7.0,
            twist: 10.0,
            hip: -2.0,
            feet: [foot(0.0, 0.31, 0.0, -10.0), foot(0.0, -0.35, 0.0, 42.0)],
            hands: [hand(0.10, -0.80, -0.36, BACK), hand(0.08, -0.72, 0.44, v3(0.3, -0.6, -1.0))],
            ..BASE
        },
        // Passada no ar: joelho da frente alto, perna de trás estendida.
        "running" => Pose {
            air: 0.07,
            lean: 17.0,
            twist: 14.0,
            head_pitch: -6.0,
            grips: [Grip::Fist, Grip::Fist],
            feet: [foot(0.0, 0.34, 0.26, 12.0), foot(0.0, -0.56, 0.12, 58.0)],
            hands: [hand(0.12, -0.40, -0.40, v3(0.2, 0.7, -1.0)), hand(0.04, -0.10, 0.50, v3(0.3, -1.0, -0.6))],
            ..BASE
        },
        "waving" => Pose {
            tilt: 1.5,
            curve: 4.0,
            hip: 4.0,
            grips: [Grip::Relaxed, Grip::Open],
            head_yaw: -10.0,
            feet: [foot(0.03, 0.02, 0.0, 0.0), foot(0.04, 0.06, 0.0, 0.0)],
            hands: [REST, hand(0.46, 0.58, 0.12, DOWN_OUT)],
            ..BASE
        },
        "celebrating" => Pose {
            lean: -5.0,
            curve: -2.0,
            grips: [Grip::Open, Grip::Open],
            head_pitch: -10.0,
            feet: [foot(0.09, 0.04, 0.0, 0.0), foot(0.08, -0.03, 0.0, 0.0)],
            hands: [hand(0.44, 0.78, 0.02, DOWN_OUT), hand(0.36, 0.84, 0.06, DOWN_OUT)],
            ..BASE
        },
        "presenting" => Pose {
            lean: 6.0,
            hip: -3.0,
            grips: [Grip::Relaxed, Grip::Open],
            crouch: 0.02,
            head_pitch: -4.0,
            feet: [foot(0.02, -0.12, 0.0, 14.0), foot(0.02, 0.20, 0.0, 0.0)],
            hands: [hand(0.16, -0.52, 0.16, v3(0.8, -0.2, -1.0)), hand(0.28, 0.06, 0.86, v3(0.5, -1.0, 0.0))],
            ..BASE
        },
        // Mão direita no queixo, braço esquerdo cruzado apoiando o cotovelo.
        "thinking" => Pose {
            tilt: -1.0,
            curve: -3.5,
            hip: -4.0,
            head_pitch: 8.0,
            head_yaw: -10.0,
            feet: [foot(0.02, -0.03, 0.0, 0.0), foot(0.03, 0.10, 0.0, 0.0)],
            hands: [hand(-0.44, -0.47, 0.30, v3(1.0, -0.5, -0.2)), hand(-0.21, 0.13, 0.26, v3(0.7, -1.0, 0.5))],
            ..BASE
        },
        "holding" => Pose {
            head_pitch: 16.0,
            curve: 2.5,
            hip: 3.0,
            grips: [Grip::Relaxed, Grip::Fist],
            hands: [REST, hand(-0.06, -0.26, 0.44, v3(0.5, -1.0, -0.4))],
            ..BASE
        },
        "reaching" => Pose {
            lean: 5.0,
            curve: -3.0,
            grips: [Grip::Relaxed, Grip::Open],
            head_pitch: -16.0,
            feet: [foot(0.0, 0.12, 0.0, 28.0), foot(0.0, -0.14, 0.0, 34.0)],
            hands: [hand(0.16, -0.78, -0.24, BACK), hand(0.10, 0.88, 0.40, v3(0.5, -0.6, -0.6))],
            ..BASE
        },
        "carrying" => Pose {
            lean: -3.0,
            grips: [Grip::Fist, Grip::Fist],
            hands: [hand(-0.06, -0.44, 0.50, v3(1.0, -0.7, -0.4)), hand(-0.06, -0.44, 0.50, v3(1.0, -0.7, -0.4))],
            ..BASE
        },
        "jumping" => Pose {
            air: 0.30,
            curve: 3.0,
            grips: [Grip::Open, Grip::Open],
            lean: -6.0,
            head_pitch: -10.0,
            feet: [foot(0.04, 0.16, 0.20, 30.0), foot(0.05, -0.30, 0.26, 60.0)],
            hands: [hand(0.36, 0.82, 0.08, DOWN_OUT), hand(0.52, 0.60, -0.06, DOWN_OUT)],
            ..BASE
        },
        "sitting" => Pose {
            seated: true,
            lean: -3.0,
            feet: [foot(0.01, 0.46, 0.0, 0.0), foot(0.01, 0.40, 0.0, 0.0)],
            hands: [hand(0.02, -0.62, 0.40, v3(0.5, -0.2, -1.0)), hand(0.02, -0.62, 0.40, v3(0.5, -0.2, -1.0))],
            ..BASE
        },
        "typing" => Pose {
            seated: true,
            grips: [Grip::Open, Grip::Open],
            lean: 5.0,
            head_pitch: 10.0,
            feet: [foot(0.01, 0.44, 0.0, 0.0), foot(0.01, 0.36, 0.0, 0.0)],
            hands: [hand(-0.14, -0.50, 0.60, v3(0.6, -1.0, -0.4)), hand(-0.14, -0.50, 0.60, v3(0.6, -1.0, -0.4))],
            ..BASE
        },
        "crouching" => Pose {
            crouch: 0.46,
            lean: 24.0,
            head_pitch: -14.0,
            feet: [foot(0.03, 0.10, 0.0, 26.0), foot(0.03, -0.06, 0.0, 44.0)],
            hands: [hand(0.02, -0.52, 0.52, v3(0.6, -0.3, -1.0)), hand(-0.02, -0.66, 0.46, v3(0.6, -0.3, -1.0))],
            ..BASE
        },
        // Encostado: peso numa perna, a outra cruzada na frente, mão na cintura.
        "leaning" => Pose {
            tilt: 5.0,
            curve: 5.0,
            hip: 6.0,
            head_yaw: -8.0,
            feet: [foot(0.02, 0.0, 0.0, 0.0), foot(-0.16, 0.14, 0.0, 46.0)],
            hands: [hand(0.12, -0.86, 0.06, BACK), hand(0.16, -0.52, -0.04, v3(1.0, 0.0, -0.9))],
            ..BASE
        },
        "desk-typing" => Pose {
            lean: 7.0,
            head_pitch: 14.0,
            feet: [foot(0.03, 0.05, 0.0, 0.0), foot(0.03, -0.05, 0.0, 0.0)],
            hands: [hand(-0.12, -0.62, 0.56, v3(0.7, -0.6, -0.8)), hand(-0.12, -0.62, 0.56, v3(0.7, -0.6, -0.8))],
            ..BASE
        },
        "hugging" => Pose {
            lean: 8.0,
            head_pitch: 6.0,
            head_yaw: 14.0,
            grips: [Grip::Open, Grip::Open],
            feet: [foot(0.03, 0.10, 0.0, 0.0), foot(0.03, -0.08, 0.0, 12.0)],
            hands: [hand(-0.20, 0.04, 0.72, v3(1.0, -0.5, 0.0)), hand(-0.24, -0.22, 0.68, v3(1.0, -0.6, 0.0))],
            ..BASE
        },
        "pointing" => Pose {
            curve: 3.0,
            hip: 3.0,
            grips: [Grip::Relaxed, Grip::Point],
            hands: [hand(0.14, -0.50, -0.02, v3(1.0, 0.0, -0.9)), hand(0.22, 0.20, 0.90, v3(0.4, -1.0, 0.0))],
            ..BASE
        },
        // Pernas cruzadas no chão.
        "floor-sitting" => Pose {
            seated: true,
            floor: true,
            lean: 4.0,
            knees: [v3(1.0, 0.55, 0.3), v3(1.0, 0.55, 0.3)],
            feet: [foot(-0.05, 0.36, 0.0, 0.0), foot(-0.05, 0.27, 0.0, 0.0)],
            hands: [hand(0.16, -0.60, 0.34, v3(0.8, -0.2, -1.0)), hand(0.16, -0.60, 0.34, v3(0.8, -0.2, -1.0))],
            ..BASE
        },
        // Deitado de costas, apoiado nos cotovelos, pernas esticadas.
        "lying" => Pose {
            seated: true,
            floor: true,
            lean: -62.0,
            head_pitch: 38.0,
            knees: [v3(0.1, 1.0, 0.2), v3(0.1, 1.0, 0.2)],
            feet: [foot(0.02, 0.80, 0.0, -62.0), foot(0.02, 0.62, 0.0, -30.0)],
            hands: [hand(0.16, -0.56, -0.10, v3(0.4, 0.2, -1.0)), hand(0.16, -0.56, -0.10, v3(0.4, 0.2, -1.0))],
            ..BASE
        },
        _ => return None,
    };
    Some(p)
}

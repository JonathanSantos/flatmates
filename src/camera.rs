//! Câmera ortográfica única: tudo o que aparece na imagem passa por ela.
use crate::math::{v3, V3};

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    /// Órbita para a direita, em radianos.
    pub yaw: f64,
    /// Inclinação para baixo, em radianos.
    pub pitch: f64,
}

pub const PRESETS: [&str; 4] = ["front", "three-quarter", "isometric", "high"];

impl Camera {
    pub fn degrees(yaw: f64, pitch: f64) -> Camera {
        Camera { yaw: yaw.to_radians(), pitch: pitch.to_radians() }
    }

    pub fn preset(name: &str) -> Option<Camera> {
        match name {
            "front" => Some(Camera::degrees(0.0, 9.0)),
            "three-quarter" => Some(Camera::degrees(26.0, 17.0)),
            "isometric" => Some(Camera::degrees(45.0, 35.264)),
            "high" => Some(Camera::degrees(18.0, 40.0)),
            _ => None,
        }
    }

    /// Mundo para espaço de vista: X à direita, Y para cima, Z em direção à câmera.
    pub fn view(&self, p: V3) -> V3 {
        p.rot_y(-self.yaw).rot_x(self.pitch)
    }

    pub fn right(&self) -> V3 {
        v3(1.0, 0.0, 0.0).rot_y(self.yaw)
    }

    pub fn up(&self) -> V3 {
        v3(0.0, 1.0, 0.0).rot_x(-self.pitch).rot_y(self.yaw)
    }

    pub fn toward(&self) -> V3 {
        v3(0.0, 0.0, 1.0).rot_x(-self.pitch).rot_y(self.yaw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_and_basis_are_inverse() {
        let cam = Camera::degrees(26.0, 17.0);
        let r = cam.view(cam.right());
        let u = cam.view(cam.up());
        let t = cam.view(cam.toward());
        assert!((r - v3(1.0, 0.0, 0.0)).len() < 1e-9);
        assert!((u - v3(0.0, 1.0, 0.0)).len() < 1e-9);
        assert!((t - v3(0.0, 0.0, 1.0)).len() < 1e-9);
    }

    #[test]
    fn nearer_ground_point_is_lower_on_screen() {
        let cam = Camera::preset("front").unwrap();
        assert!(cam.view(v3(0.0, 0.0, 1.0)).y < cam.view(v3(0.0, 0.0, 0.0)).y);
    }
}

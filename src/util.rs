use std::f32::consts::PI;

use macroquad::math::Rect;
use macroquad::prelude::{Color, draw_rectangle};

/// Clamps the angle to range -PI to PI
pub fn normalize_angle(angle: f32) -> f32 {
    let mut new_angle = angle;
    while new_angle < -PI {
        new_angle += 2. * PI;
    }
    while new_angle > PI {
        new_angle -= 2.0 * PI;
    }
    new_angle
}

pub trait RectExtensions {
    fn draw_rectangle(&self, color: Color);
}

impl RectExtensions for Rect {
    fn draw_rectangle(&self, color: Color) {
        draw_rectangle(self.x, self.y, self.w, self.h, color)
    }
}

#[test]
fn test_normalize_angle() {
    assert_eq!(normalize_angle(PI), PI);
    assert_eq!(normalize_angle(PI * 2.), 0.);
    assert_eq!(normalize_angle(-PI), -PI);
    assert_eq!(normalize_angle(-PI * 2.), 0.);
    assert_eq!(normalize_angle(-PI - 0.1), PI - 0.1);
    assert_eq!(normalize_angle(PI + 0.1), -PI + 0.1);
}

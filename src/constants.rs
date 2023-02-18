use std::f32::consts::PI;

pub const WHEEL_DRAG: f32 = 512.0;
pub const FIELD_OF_VIEW: f32 = 45.0 * PI / 180.0;
pub const AMORTIZATION: f32 = 0.95;
pub const VERTICES_PER_FACET: i32 = 3;

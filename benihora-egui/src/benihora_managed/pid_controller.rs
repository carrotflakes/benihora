use serde::{Deserialize, Serialize};

pub struct PIDController {
    pub(super) dtime: f32,
    integral: f32,
    last: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PIDParam {
    pub kp: f32,
    pub ki: f32,
    pub kd: f32,
}

impl PIDParam {
    pub fn new(kp: f32, ki: f32, kd: f32) -> Self {
        Self { kp, ki, kd }
    }
}

impl PIDController {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            dtime: 1.0 / sample_rate,
            integral: 0.0,
            last: 0.0,
        }
    }

    pub fn process(&mut self, pid: &PIDParam, x: f32) -> f32 {
        let d = x - self.last;
        self.integral = self.integral + x * self.dtime;
        let y = (x * pid.kp + self.integral * pid.ki) * self.dtime + d * pid.kd;
        self.last = x;
        y
    }
}

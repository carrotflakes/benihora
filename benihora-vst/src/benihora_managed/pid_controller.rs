use serde::{Deserialize, Serialize};

pub struct PIDController {
    pub(super) dtime: f64,
    integral: f64,
    last: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PIDParam {
    pub kp: f64,
    pub ki: f64,
    pub kd: f64,
}

impl PIDParam {
    pub fn new(kp: f64, ki: f64, kd: f64) -> Self {
        Self { kp, ki, kd }
    }
}

impl PIDController {
    pub fn new(sample_rate: f64) -> Self {
        Self {
            dtime: 1.0 / sample_rate,
            integral: 0.0,
            last: 0.0,
        }
    }

    pub fn process(&mut self, pid: &PIDParam, x: f64) -> f64 {
        let d = x - self.last;
        self.integral = self.integral + x * self.dtime;
        let y = (x * pid.kp + self.integral * pid.ki) * self.dtime + d * pid.kd;
        self.last = x;
        y
    }
}

use benihora::tract::DEFAULT_TONGUE;

pub struct Tract {
    pub tongue_target: (f64, f64),
    pub speed: f64,
}

impl Tract {
    pub fn new() -> Self {
        Self {
            tongue_target: DEFAULT_TONGUE,
            speed: 20.0,
        }
    }

    pub fn update(&mut self, dtime: f64, tongue: &mut (f64, f64)) {
        let x_scale = 8.0;
        let x = (self.tongue_target.0 - tongue.0) / x_scale;
        let y = self.tongue_target.1 - tongue.1;
        let d = x.hypot(y);
        if d < 0.0001 {
            return;
        }
        let dx = x / d * dtime * self.speed * x_scale;
        let dy = y / d * dtime * self.speed;

        tongue.0 = if self.tongue_target.0 < tongue.0 {
            self.tongue_target.0.max(tongue.0 + dx)
        } else {
            self.tongue_target.0.min(tongue.0 + dx)
        };
        tongue.1 = if self.tongue_target.1 < tongue.1 {
            self.tongue_target.1.max(tongue.1 + dy)
        } else {
            self.tongue_target.1.min(tongue.1 + dy)
        };
    }
}

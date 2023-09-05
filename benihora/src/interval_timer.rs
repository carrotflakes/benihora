pub struct IntervalTimer {
    pub interval: f32,
    pub time: f32,
    pub overflowed: bool,
}

impl IntervalTimer {
    pub fn new(interval: f32) -> Self {
        Self {
            interval,
            time: 0.0,
            overflowed: false,
        }
    }

    pub fn new_overflowed(interval: f32) -> Self {
        Self {
            interval,
            time: 0.0,
            overflowed: true,
        }
    }

    pub fn time(&self) -> f32 {
        self.time
    }

    pub fn overflowed(&self) -> bool {
        self.overflowed
    }

    pub fn progress(&self) -> f32 {
        self.time / self.interval
    }

    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        self.overflowed = self.time >= self.interval;
        self.time %= self.interval;
    }
}

use crate::F;

pub struct IntervalTimer {
    pub interval: F,
    pub time: F,
    pub overflowed: bool,
}

impl IntervalTimer {
    pub fn new(interval: F) -> Self {
        Self {
            interval,
            time: 0.0,
            overflowed: false,
        }
    }

    pub fn new_overflowed(interval: F) -> Self {
        Self {
            interval,
            time: 0.0,
            overflowed: true,
        }
    }

    pub fn time(&self) -> F {
        self.time
    }

    pub fn overflowed(&self) -> bool {
        self.overflowed
    }

    pub fn progress(&self) -> F {
        self.time / self.interval
    }

    pub fn update(&mut self, dt: F) {
        self.time += dt;
        self.overflowed = self.time >= self.interval;
        self.time %= self.interval;
    }
}

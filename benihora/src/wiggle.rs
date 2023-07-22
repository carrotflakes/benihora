use crate::rand_f64;

pub struct Wiggle {
    frequency: f64,
    rand: u32,
    current_value: f64,
    next_value: f64,
    dvalue: f64,
    current_frequency: f64,
    time: f64,
    dtime: f64,
}

impl Wiggle {
    pub fn new(dtime: f64, frequency: f64, mut seed: u32) -> Self {
        assert_ne!(seed, 0);
        assert!(dtime * frequency < 0.5);
        let current_frequency = frequency * (rand_f64(&mut seed) + 0.5);
        Wiggle {
            frequency,
            current_value: 0.0,
            next_value: rand_f64(&mut seed) * 2.0 - 1.0,
            dvalue: 0.0,
            current_frequency,
            time: 1.0 / current_frequency,
            rand: seed,
            dtime,
        }
    }

    pub fn process(&mut self) -> f64 {
        let factor = self.dtime * self.current_frequency;
        self.dvalue =
            self.dvalue * (1.0 - factor) + (self.next_value - self.current_value) * factor * factor;
        self.current_value = self.current_value + self.dvalue;
        self.time -= self.dtime;
        if self.time < 0.0 {
            self.current_frequency = self.frequency * (rand_f64(&mut self.rand) + 0.5);
            self.time = 1.0 / self.current_frequency;
            self.next_value = rand_f64(&mut self.rand) * 2.0 - 1.0;
        }
        self.current_value
    }
}

#[test]
fn test() {
    let mut wiggle = Wiggle::new(0.02, 40.9, 1);
    for _ in 0..100 {
        dbg!(wiggle.process());
    }
}

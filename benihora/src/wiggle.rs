use crate::rand_f32;

pub struct Wiggle {
    frequency: f32,
    rand: u32,
    current_value: f32,
    next_value: f32,
    dvalue: f32,
    current_frequency: f32,
    time: f32,
    dtime: f32,
}

impl Wiggle {
    pub fn new(dtime: f32, frequency: f32, mut seed: u32) -> Self {
        assert_ne!(seed, 0);
        assert!(dtime * frequency < 0.5);
        let current_frequency = frequency * (rand_f32(&mut seed) + 0.5);
        Wiggle {
            frequency,
            current_value: 0.0,
            next_value: rand_f32(&mut seed) * 2.0 - 1.0,
            dvalue: 0.0,
            current_frequency,
            time: 1.0 / current_frequency,
            rand: seed,
            dtime,
        }
    }

    pub fn process(&mut self) -> f32 {
        let factor = self.dtime * self.current_frequency;
        self.dvalue =
            self.dvalue * (1.0 - factor) + (self.next_value - self.current_value) * factor * factor;
        self.current_value = self.current_value + self.dvalue;
        self.time -= self.dtime;
        if self.time < 0.0 {
            self.current_frequency = self.frequency * (rand_f32(&mut self.rand) + 0.5);
            self.time = 1.0 / self.current_frequency;
            self.next_value = rand_f32(&mut self.rand) * 2.0 - 1.0;
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

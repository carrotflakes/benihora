use std::f32::consts::TAU;

use crate::{lerp, wiggle::Wiggle, Benihora, IntervalTimer};

pub struct BenihoraManaged {
    pub sound: bool,
    pub frequency: Frequency,
    tenseness: Tenseness,
    pub intensity: Intensity,
    loudness: Loudness,
    pub benihora: Benihora,
    update_timer: IntervalTimer,
    dtime: f32,
}

impl BenihoraManaged {
    pub fn new(sound_speed: f32, sample_rate: f32, seed: u32) -> Self {
        assert!(seed < 1 << 16);
        let interval = 0.02;
        Self {
            sound: false,
            frequency: Frequency::new(interval, seed, 140.0, 0.005, 6.0),
            tenseness: Tenseness::new(interval, seed + 2, 0.6),
            intensity: Intensity::new(0.0),
            loudness: Loudness::new(0.6f32.powf(0.25)),
            benihora: Benihora::new(sound_speed, sample_rate, 1.0, seed, true),
            update_timer: IntervalTimer::new_overflowed(interval),
            dtime: 1.0 / sample_rate,
        }
    }

    /// let v = v.clamp(0.0, 1.0);
    /// set_tenseness(1.0 - (v * std::f32::consts::PI * 0.5).cos());
    pub fn set_tenseness(&mut self, tenseness: f32) {
        let tenseness = tenseness.clamp(0.0, 1.0);
        self.tenseness.target_tenseness = tenseness;
        self.loudness.target = tenseness.powf(0.25);
    }

    pub fn process(&mut self, current_time: f32) -> f32 {
        if self.update_timer.overflowed() {
            self.intensity
                .update(self.sound, self.update_timer.interval);
            self.frequency.update(current_time);
            self.tenseness.update();
        }
        let lambda = self.update_timer.progress();
        self.update_timer.update(self.dtime);

        let intensity = self.intensity.get(lambda);
        let frequency = self.frequency.get(lambda);
        let tenseness = self.tenseness.get(lambda);
        let loudness = self.loudness.process(self.dtime);
        self.benihora
            .process(frequency, tenseness, intensity, loudness, 1.0)
    }
}

pub struct Frequency {
    old_frequency: f32,
    new_frequency: f32,
    pub target_frequency: f32,
    smooth_frequency: f32,

    pub vibrato_amount: f32,
    pub vibrato_frequency: f32,
    pub wobble_amount: f32,
    wiggles: [Wiggle; 2],
}

impl Frequency {
    pub fn new(
        dtime: f32,
        seed: u32,
        frequency: f32,
        vibrato_amount: f32,
        vibrato_frequency: f32,
    ) -> Self {
        Self {
            old_frequency: frequency,
            new_frequency: frequency,
            target_frequency: frequency,
            smooth_frequency: frequency,
            vibrato_amount,
            vibrato_frequency,
            wobble_amount: 1.0,
            wiggles: [
                Wiggle::new(dtime / 4.0, 4.07 * 5.0, seed + 1),
                Wiggle::new(dtime / 4.0, 2.15 * 5.0, seed + 2),
            ],
        }
    }

    pub fn set(&mut self, frequency: f32) {
        self.target_frequency = frequency;
    }

    pub fn update(&mut self, time: f32) {
        let mut vibrato = self.vibrato_amount * (TAU * time * self.vibrato_frequency).sin();
        vibrato += self.wobble_amount
            * (0.01 * self.wiggles[0].process() + 0.02 * self.wiggles[1].process());
        for _ in 0..3 {
            self.wiggles[0].process();
            self.wiggles[1].process();
        }

        self.smooth_frequency = (self.smooth_frequency + self.target_frequency) * 0.5;

        self.old_frequency = self.new_frequency;
        self.new_frequency = self.smooth_frequency * (1.0 + vibrato);
    }

    pub fn get(&self, lambda: f32) -> f32 {
        lerp(self.old_frequency, self.new_frequency, lambda)
    }
}

pub struct Tenseness {
    old_tenseness: f32,
    new_tenseness: f32,
    pub target_tenseness: f32,
    wiggles: [Wiggle; 2],
    pub wobble_amount: f32,
}

impl Tenseness {
    pub fn new(dtime: f32, seed: u32, tenseness: f32) -> Self {
        Self {
            old_tenseness: tenseness,
            new_tenseness: tenseness,
            target_tenseness: tenseness,
            wiggles: [
                Wiggle::new(dtime, 0.46 * 5.0, seed + 1),
                Wiggle::new(dtime, 0.36 * 5.0, seed + 2),
            ],
            wobble_amount: 1.0,
        }
    }

    pub fn update(&mut self) {
        self.old_tenseness = self.new_tenseness;
        self.new_tenseness = self.target_tenseness
            + (0.05 * self.wiggles[0].process() + 0.025 * self.wiggles[1].process())
                * self.wobble_amount;
        self.new_tenseness = self.new_tenseness.clamp(0.0, 1.0);
    }

    pub fn get(&self, lambda: f32) -> f32 {
        lerp(self.old_tenseness, self.new_tenseness, lambda)
    }
}

pub struct Intensity {
    old_intensity: f32,
    new_intensity: f32,
    pub up_velocity: f32,
    pub down_velocity: f32,
}

impl Intensity {
    pub fn new(intensity: f32) -> Self {
        Self {
            old_intensity: intensity,
            new_intensity: intensity,
            up_velocity: 3.25,
            down_velocity: 5.0,
        }
    }

    pub fn update(&mut self, sound: bool, interval: f32) {
        self.old_intensity = self.new_intensity;
        if sound {
            self.new_intensity += interval * self.up_velocity;
        } else {
            self.new_intensity -= interval * self.down_velocity;
        }
        self.new_intensity = self.new_intensity.clamp(0.0, 1.0);
    }

    pub fn get(&self, lambda: f32) -> f32 {
        lerp(self.old_intensity, self.new_intensity, lambda)
    }
}

pub struct Loudness {
    current: f32,
    pub target: f32,
}

impl Loudness {
    pub fn new(loudness: f32) -> Self {
        Self {
            current: loudness,
            target: loudness,
        }
    }

    pub fn process(&mut self, dtime: f32) -> f32 {
        self.current = if self.current < self.target {
            self.target.min(self.current + 10.0 * dtime)
        } else {
            self.target.max(self.current - 10.0 * dtime)
        };
        self.current
    }
}

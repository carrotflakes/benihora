mod pid_controller;
mod tract;

use std::f64::consts::TAU;

use benihora::{
    lerp,
    managed::{Loudness, Tenseness},
    wiggle::Wiggle,
    Benihora, IntervalTimer,
};
use serde::{Deserialize, Serialize};

use crate::waveform_recorder::WaveformRecorder;

pub struct BenihoraManaged {
    pub sound: bool,
    pub frequency: Frequency,
    pub tenseness: Tenseness,
    pub intensity: Intensity,
    pub loudness: Loudness,
    pub tract: tract::Tract,
    pub benihora: Benihora,
    update_timer: IntervalTimer,
    sample_rate: f64,
    dtime: f64,
    pub history: Vec<[f32; 5]>,
    pub history_count: usize,
    pub level: f32,
    pub waveform_recorder: WaveformRecorder,
}

#[derive(Serialize, Deserialize)]
pub struct Params {
    pub always_sound: bool,
    pub frequency_pid: pid_controller::PIDParam,
    pub intensity_pid: pid_controller::PIDParam,
    pub wobble_amount: f64,
    pub vibrato_amount: f64,
    pub vibrato_frequency: f64,
    pub aspiration_level: f64,
}

impl Params {
    pub fn new() -> Self {
        Self {
            always_sound: false,
            frequency_pid: pid_controller::PIDParam::new(50.0, 20.0, 0.3),
            intensity_pid: pid_controller::PIDParam::new(10.0, 100.0, 0.0), // recomend kd = 0.0
            wobble_amount: 0.1,
            vibrato_amount: 0.005,
            vibrato_frequency: 6.0,
            aspiration_level: 1.0,
        }
    }
}

impl BenihoraManaged {
    pub fn new(sound_speed: f64, sample_rate: f64, over_sample: f64, seed: u32) -> Self {
        let interval = 0.02;
        Self {
            sound: false,
            frequency: Frequency::new(interval, seed, 140.0, sample_rate),
            tenseness: Tenseness::new(interval, seed, 0.6),
            intensity: Intensity::new(sample_rate),
            loudness: Loudness::new(0.6f64.powf(0.25)),
            tract: tract::Tract::new(),
            benihora: Benihora::new(sound_speed, sample_rate, over_sample, seed, false),
            update_timer: IntervalTimer::new_overflowed(interval),
            sample_rate,
            dtime: 1.0 / sample_rate,
            history: Vec::new(),
            history_count: 0,
            level: 0.0,
            waveform_recorder: WaveformRecorder::new(),
        }
    }

    pub fn set_tenseness(&mut self, tenseness: f64) {
        let tenseness = tenseness.clamp(0.0, 1.0);
        self.tenseness.target_tenseness = tenseness;
        self.loudness.target = tenseness.powf(0.25);
    }

    pub fn process(&mut self, params: &Params) -> f64 {
        if self.update_timer.overflowed() {
            self.frequency.update(
                self.update_timer.interval,
                params.wobble_amount,
                params.vibrato_amount,
                params.vibrato_frequency,
            );
            self.tenseness.update();
            self.tract.update(
                self.update_timer.interval,
                &mut self.benihora.tract.source.tongue,
            );
            self.benihora.tract.update_diameter();
        }
        let lambda = self.update_timer.progress();
        self.update_timer.update(self.dtime);

        let intensity = self.intensity.process(
            &params.intensity_pid,
            if self.sound | params.always_sound {
                1.0
            } else {
                0.0
            },
        );
        let frequency = self.frequency.get(&params.frequency_pid, lambda);
        let tenseness = self.tenseness.get(lambda);
        let loudness = self.loudness.process(self.dtime);

        if self.history_count == 0 {
            self.history_count = self.sample_rate as usize / 50;
            self.history.push([
                frequency as f32,
                intensity as f32,
                tenseness as f32,
                loudness as f32,
                (self.level / self.history_count as f32).sqrt(),
            ]);
            self.level = 0.0;
            if self.history.len() > 1000 {
                self.history.remove(0);
            }
        }
        self.history_count -= 1;
        self.level += self.benihora.get_glottal_output().powi(2) as f32;

        let y = self.benihora.process(
            frequency,
            tenseness,
            intensity,
            loudness,
            params.aspiration_level,
        );

        self.waveform_recorder.record(
            self.benihora.glottis.get_phase(),
            self.benihora.get_glottal_output(),
        );

        y
    }
}

pub struct Frequency {
    value: f64,
    pid: pid_controller::PIDController,
    old_vibrate: f64,
    new_vibrate: f64,
    target_frequency: f64,
    pub pitchbend: f64,
    phase: f64,

    wiggles: [Wiggle; 2],
}

impl Frequency {
    pub fn new(dtime: f64, seed: u32, frequency: f64, sample_rate: f64) -> Self {
        Self {
            value: frequency,
            pid: pid_controller::PIDController::new(sample_rate),
            old_vibrate: 1.0,
            new_vibrate: 1.0,
            target_frequency: frequency,
            pitchbend: 1.0,
            phase: (seed as f64 / 10.0) % 1.0,
            wiggles: [
                Wiggle::new(dtime / 4.0, 4.07 * 5.0, seed + 1),
                Wiggle::new(dtime / 4.0, 2.15 * 5.0, seed + 2),
            ],
        }
    }

    pub fn set(&mut self, frequency: f64, reset: bool) {
        self.target_frequency = frequency;
        if reset {
            self.value = frequency;
        }
    }

    fn update(
        &mut self,
        dtime: f64,
        wobble_amount: f64,
        vibrato_amount: f64,
        vibrato_frequency: f64,
    ) {
        let mut vibrato = vibrato_amount * (TAU * self.phase).sin();
        self.phase = (self.phase + dtime * vibrato_frequency) % 1.0;
        vibrato +=
            wobble_amount * (0.01 * self.wiggles[0].process() + 0.02 * self.wiggles[1].process());
        for _ in 0..3 {
            self.wiggles[0].process();
            self.wiggles[1].process();
        }

        self.old_vibrate = self.new_vibrate;
        self.new_vibrate = 1.0 + vibrato;
    }

    pub fn get(&mut self, pid: &pid_controller::PIDParam, lambda: f64) -> f64 {
        let vibrate = lerp(self.old_vibrate, self.new_vibrate, lambda);
        let target_frequency = self.target_frequency * vibrate * self.pitchbend;
        // self.value *= self.pid.process(target_frequency / self.value - 1.0) + 1.0;
        self.value += self.pid.process(pid, target_frequency - self.value);
        self.value = self.value.clamp(10.0, 10000.0);
        self.value
    }
}

pub struct Intensity {
    value: f64,
    bias: f64,
    pid: pid_controller::PIDController,
}

impl Intensity {
    pub fn new(sample_rate: f64) -> Self {
        Self {
            value: 0.0,
            bias: -1.0,
            pid: pid_controller::PIDController::new(sample_rate),
        }
    }

    pub fn get(&self) -> f64 {
        self.value
    }

    pub fn process(&mut self, pid: &pid_controller::PIDParam, target: f64) -> f64 {
        self.value += self.pid.process(pid, target - self.value) + self.bias * self.pid.dtime;
        self.value = self.value.max(0.0);
        self.value
    }
}

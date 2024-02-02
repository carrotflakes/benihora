mod pid_controller;
mod tract;

use std::f32::consts::TAU;

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
    intensity_adsr: IntensityAdsr,
    intensity_pid: IntensityPid,
    pub intensity_pid_enabled: bool,
    pub loudness: Loudness,
    pub tract: tract::Tract,
    pub benihora: Benihora,
    update_timer: IntervalTimer,
    sample_rate: f32,
    dtime: f32,
    pub history: Vec<[f32; 5]>,
    pub history_count: usize,
    pub level: f32,
    pub waveform_recorder: WaveformRecorder,
}

#[derive(Serialize, Deserialize)]
pub struct Params {
    pub always_sound: bool,
    pub frequency_pid: pid_controller::PIDParam,
    pub intensity_adsr: [f32; 4],
    pub intensity_pid: pid_controller::PIDParam,
    pub noteon_intensity: f32,
    pub frequency_wobble_amount: f32,
    pub vibrato_amount: f32,
    pub vibrato_rate: f32,
    pub tenseness_wobble_amount: f32,
    pub aspiration_level: f32,
}

impl Params {
    pub fn new() -> Self {
        Self {
            always_sound: false,
            frequency_pid: pid_controller::PIDParam::new(50.0, 20.0, 0.0),
            intensity_adsr: [0.02, 0.05, 0.75, 0.05],
            intensity_pid: pid_controller::PIDParam::new(10.0, 100.0, 0.0), // recomend kd = 0.0
            noteon_intensity: 0.9,
            frequency_wobble_amount: 0.1,
            vibrato_amount: 0.005,
            vibrato_rate: 6.0,
            tenseness_wobble_amount: 1.0,
            aspiration_level: 1.0,
        }
    }
}

impl BenihoraManaged {
    pub fn new(sound_speed: f32, sample_rate: f32, over_sample: f32, seed: u32) -> Self {
        let interval = 0.02;
        Self {
            sound: false,
            frequency: Frequency::new(interval, seed, 140.0, 1.0 / interval),
            tenseness: Tenseness::new(interval, seed, 0.6),
            intensity_pid: IntensityPid::new(sample_rate),
            intensity_adsr: IntensityAdsr::new(sample_rate),
            intensity_pid_enabled: false,
            loudness: Loudness::new(0.6f32.powf(0.25)),
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

    pub fn set_tenseness(&mut self, tenseness: f32) {
        let tenseness = tenseness.clamp(0.0, 1.0);
        self.tenseness.target_tenseness = tenseness;
        self.loudness.target = tenseness.powf(0.25);
    }

    pub fn get_intensity(&self) -> f32 {
        if self.intensity_pid_enabled {
            self.intensity_pid.get()
        } else {
            self.intensity_adsr.get()
        }
    }

    pub fn process(&mut self, params: &Params) -> f32 {
        self.tenseness.wobble_amount = params.tenseness_wobble_amount;

        if self.update_timer.overflowed() {
            self.frequency.update(
                self.update_timer.interval,
                params.frequency_wobble_amount,
                params.vibrato_amount,
                params.vibrato_rate,
                &params.frequency_pid,
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

        let intensity = if self.intensity_pid_enabled {
            self.intensity_pid.process(
                &params.intensity_pid,
                if self.sound | params.always_sound {
                    params.noteon_intensity
                } else {
                    0.0
                },
            )
        } else {
            self.intensity_adsr
                .process(&params.intensity_adsr, self.sound | params.always_sound)
                * params.noteon_intensity
        };
        let frequency = self.frequency.get(lambda);
        let tenseness = self.tenseness.get(lambda);
        let loudness = self.loudness.process(self.dtime);

        if self.history_count == 0 {
            self.history_count = self.sample_rate as usize / 50;
            self.history.push([
                frequency,
                intensity,
                tenseness,
                loudness,
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
    pid: pid_controller::PIDController,
    old_frequency: f32,
    new_frequency: f32,
    target_frequency: f32,
    pub pitchbend: f32,
    phase: f32,

    wiggles: [Wiggle; 2],
}

impl Frequency {
    pub fn new(dtime: f32, seed: u32, frequency: f32, update_rate: f32) -> Self {
        Self {
            pid: pid_controller::PIDController::new(update_rate),
            old_frequency: frequency,
            new_frequency: frequency,
            target_frequency: frequency,
            pitchbend: 1.0,
            phase: (seed as f32 / 10.0) % 1.0,
            wiggles: [
                Wiggle::new(dtime / 4.0, 4.07 * 5.0, seed + 1),
                Wiggle::new(dtime / 4.0, 2.15 * 5.0, seed + 2),
            ],
        }
    }

    pub fn set(&mut self, frequency: f32, reset: bool) {
        self.target_frequency = frequency;
        if reset {
            self.old_frequency = frequency;
            self.new_frequency = frequency;
        }
    }

    fn update(
        &mut self,
        dtime: f32,
        wobble_amount: f32,
        vibrato_amount: f32,
        vibrato_frequency: f32,
        pid: &pid_controller::PIDParam,
    ) {
        let mut vibrato = vibrato_amount * (TAU * self.phase).sin();
        self.phase = (self.phase + dtime * vibrato_frequency) % 1.0;
        vibrato +=
            wobble_amount * (0.01 * self.wiggles[0].process() + 0.02 * self.wiggles[1].process());
        for _ in 0..3 {
            self.wiggles[0].process();
            self.wiggles[1].process();
        }

        self.old_frequency = self.new_frequency;
        let target_frequency = self.target_frequency * (1.0 + vibrato);
        self.new_frequency *= self
            .pid
            .process(pid, (target_frequency / self.new_frequency).ln())
            .exp();
        self.new_frequency *= self.pitchbend;
        self.new_frequency = self.new_frequency.clamp(10.0, 10000.0);
    }

    pub fn get(&mut self, lambda: f32) -> f32 {
        lerp(self.old_frequency, self.new_frequency, lambda)
    }
}

pub struct IntensityPid {
    value: f32,
    bias: f32,
    pid: pid_controller::PIDController,
}

impl IntensityPid {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            value: 0.0,
            bias: -1.0,
            pid: pid_controller::PIDController::new(sample_rate),
        }
    }

    pub fn get(&self) -> f32 {
        self.value
    }

    pub fn process(&mut self, pid: &pid_controller::PIDParam, target: f32) -> f32 {
        self.value += self.pid.process(pid, target - self.value) + self.bias * self.pid.dtime;
        self.value = self.value.max(0.0);
        self.value
    }
}

pub struct IntensityAdsr {
    elapsed: f32,
    dtime: f32,
    value: f32,
}

impl IntensityAdsr {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            elapsed: 0.0,
            dtime: 1.0 / sample_rate,
            value: 0.0,
        }
    }

    pub fn get(&self) -> f32 {
        self.value
    }

    pub fn process(&mut self, adsr: &[f32; 4], sound: bool) -> f32 {
        if sound {
            self.elapsed += self.dtime;
            self.value = if self.elapsed < adsr[0] {
                self.value.max(self.elapsed / adsr[0])
            } else if self.elapsed < adsr[0] + adsr[1] {
                lerp(1.0, adsr[2], (self.elapsed - adsr[0]) / adsr[1])
            } else {
                adsr[2]
            };
        } else {
            self.elapsed = 0.0;
            self.value = if self.value > 0.0 {
                self.value - self.dtime / adsr[3]
            } else {
                0.0
            };
        }
        self.value
    }
}

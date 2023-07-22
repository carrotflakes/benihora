use crate::resample::Resample;

use super::glottis::Glottis;
use super::tract::Tract;
use super::F;

pub struct Benihora {
    force_turbulence: bool,
    pub sample_rate: F,
    pub glottis: Glottis,
    pub tract: Tract,
    resample: Resample,
    glottal_output: F,
}

impl Benihora {
    pub fn new(
        sound_speed: F,
        sample_rate: F,
        over_sample: F,
        seed: u32,
        force_turbulence: bool,
    ) -> Self {
        assert!(seed < u32::MAX - 2);

        let tract_steps = 48000.0 * sound_speed;
        let tract_steps_per_process = ((tract_steps / sample_rate) as usize).max(1);
        let inner_sample_rate = tract_steps / tract_steps_per_process as F * over_sample;

        Self {
            force_turbulence,
            sample_rate,
            glottis: Glottis::new(inner_sample_rate, seed),
            tract: Tract::new(tract_steps_per_process, inner_sample_rate, seed + 1),
            resample: Resample::new(inner_sample_rate, sample_rate),
            glottal_output: 0.0,
        }
    }

    pub fn get_glottal_output(&self) -> F {
        self.glottal_output
    }

    pub fn process(
        &mut self,
        frequency: F,
        tenseness: F,
        intensity: F,
        loudness: F,
        aspiration_level: F,
    ) -> F {
        debug_assert!((1.0..=10000.0).contains(&frequency));
        debug_assert!((0.0..=1.0).contains(&tenseness));
        debug_assert!((0.0..=1.0).contains(&intensity));
        debug_assert!((0.0..=1.0).contains(&loudness));

        let tract_intensity = if self.force_turbulence {
            1.0
        } else {
            intensity
        };

        self.resample.process(|| {
            self.glottal_output =
                self.glottis
                    .process(frequency, tenseness, intensity, loudness, aspiration_level);

            self.tract.process(tract_intensity, self.glottal_output)
        })
    }
}

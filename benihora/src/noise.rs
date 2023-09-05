use biquad::{Biquad, Coefficients, DirectForm2Transposed, ToHertz};

use crate::rand_f32;

pub struct Noise {
    rand: u32,
    filter: DirectForm2Transposed<f32>,
}

impl Noise {
    pub fn new(seed: u32, sample_rate: f32, frequency: f32) -> Self {
        assert!(seed != 0);
        Self {
            rand: seed,
            filter: DirectForm2Transposed::<f32>::new(
                Coefficients::<f32>::from_params(
                    biquad::Type::BandPass,
                    sample_rate.hz(),
                    frequency.hz(),
                    0.5,
                )
                .unwrap(),
            ),
        }
    }

    pub fn process(&mut self) -> f32 {
        let x = rand_f32(&mut self.rand);
        self.filter.run(x * 2.0 - 1.0)
    }
}

use biquad::{Biquad, Coefficients, DirectForm2Transposed, ToHertz};

use crate::{rand_f64, F};

pub struct Noise {
    rand: u32,
    filter: DirectForm2Transposed<F>,
}

impl Noise {
    pub fn new(seed: u32, sample_rate: F, frequency: F) -> Self {
        assert!(seed != 0);
        Self {
            rand: seed,
            filter: DirectForm2Transposed::<F>::new(
                Coefficients::<F>::from_params(
                    biquad::Type::BandPass,
                    sample_rate.hz(),
                    frequency.hz(),
                    0.5,
                )
                .unwrap(),
            ),
        }
    }

    pub fn process(&mut self) -> F {
        let x = rand_f64(&mut self.rand);
        self.filter.run(x * 2.0 - 1.0)
    }
}

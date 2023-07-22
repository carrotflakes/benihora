#[derive(Debug, Clone)]
pub enum Algo {
    UpSample {
        in_per_out: f64,
        prev_sample: f64,
        next_sample: f64,
        next_sample_time: f64,
    },
    DownSample {
        in_per_out: f64,
        out_per_in: f64,
        left_value: f64,
        right_value: f64,
        time: f64,
    },
    Identity,
}

pub struct Resample {
    algo: Algo,
}

impl Resample {
    pub fn new(input_sample_rate: f64, output_sample_rate: f64) -> Self {
        Self {
            algo: if input_sample_rate < output_sample_rate {
                Algo::UpSample {
                    in_per_out: input_sample_rate / output_sample_rate,
                    prev_sample: 0.0,
                    next_sample: 0.0,
                    next_sample_time: 1.0,
                }
            } else if input_sample_rate > output_sample_rate {
                Algo::DownSample {
                    in_per_out: input_sample_rate / output_sample_rate,
                    out_per_in: output_sample_rate / input_sample_rate,
                    left_value: 0.0,
                    right_value: 0.0,
                    time:0.0,
                }
            } else {
                Algo::Identity
            },
        }
    }

    pub fn process(&mut self, mut x: impl FnMut() -> f64) -> f64 {
        match self.algo {
            Algo::UpSample {
                in_per_out,
                ref mut prev_sample,
                ref mut next_sample,
                ref mut next_sample_time,
            } => {
                *next_sample_time = *next_sample_time + in_per_out;
                while 1.0 <= *next_sample_time {
                    *next_sample_time = *next_sample_time - 1.0;
                    *prev_sample = *next_sample;
                    *next_sample = x();
                }
                let t = *next_sample_time;
                *prev_sample + (*next_sample - *prev_sample) * t
            }
            Algo::DownSample {
                in_per_out,
                out_per_in,
                ref mut left_value,
                ref mut right_value,
                ref mut time,
            } => {
                *time = *time + in_per_out;
                let y = *left_value;
                *left_value = *right_value;
                while 1.0 <= *time {
                    *left_value = *left_value + x();
                    *time = *time - 1.0;
                }
                let x = x();
                *left_value = *left_value + x * *time;
                *right_value = x * (1.0 - *time);
                *time = *time - 1.0;
                y * out_per_in
            }
            Algo::Identity => x(),
        }
    }
}

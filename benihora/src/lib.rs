mod benihora;
pub mod glottis;
mod interval_timer;
pub mod managed;
mod noise;
pub mod resample;
pub mod tract;
pub mod wiggle;

pub use self::benihora::Benihora;
pub use glottis::Glottis;
pub use interval_timer::IntervalTimer;
pub use managed::BenihoraManaged;

type F = f64;

#[inline]
pub fn lerp(a: F, b: F, t: F) -> F {
    a + (b - a) * t
}

pub fn rand_f64(seed: &mut u32) -> f64 {
    *seed = seed.overflowing_mul(48271).0 % ((1 << 31) - 1);
    (*seed << 1) as f64 / std::u32::MAX as f64
}

pub fn tract_impulse_response(n: usize, tract: &tract::Tract) -> Vec<F> {
    let mut state = tract::State::new(tract.source.length, tract.source.nose_length);
    impulse_response(n, |x| {
        let lip_output = state.process_mouth(
            &tract.params,
            &tract.new_reflections,
            &tract.new_reflections,
            0.0,
            x,
        );
        let nose_out = state.process_nose(
            &tract.params,
            &tract.new_reflections,
            tract.new_reflections.nose[0],
        );
        lip_output + nose_out
    })
}

pub fn impulse_response(n: usize, mut f: impl FnMut(f64) -> f64) -> Vec<f64> {
    let mut buffer = Vec::with_capacity(n);
    buffer.push(f(1.0));
    for _ in 1..n {
        buffer.push(f(0.0));
    }
    buffer
}

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

#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub fn rand_f32(seed: &mut u32) -> f32 {
    *seed = seed.overflowing_mul(48271).0 % ((1 << 31) - 1);
    (*seed << 1) as f32 / std::u32::MAX as f32
}

pub fn tract_impulse_response(n: usize, benihora: &Benihora) -> (Vec<f32>, f32) {
    let tract = &benihora.tract;
    let mut state = tract::State::new(tract.source.length, tract.source.nose_length);

    let response = impulse_response(n, |x| {
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
    });

    (
        response,
        benihora.inner_sample_rate * tract.steps_per_process as f32,
    )
}

pub fn impulse_response(n: usize, mut f: impl FnMut(f32) -> f32) -> Vec<f32> {
    let mut buffer = Vec::with_capacity(n);
    buffer.push(f(1.0));
    for _ in 1..n {
        buffer.push(f(0.0));
    }
    buffer
}

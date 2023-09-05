use std::f32::consts::PI;

use crate::{lerp, noise::Noise, wiggle::Wiggle};

pub struct Glottis {
    pub(crate) aspiration_noise: Noise,
    phase: f32,
    waveform: WaveformIntegral,
    sample_rate: f32,
    wiggle: Wiggle,
    last_integral: f32,
}

impl Glottis {
    pub fn new(sample_rate: f32, seed: u32) -> Self {
        let waveform = WaveformIntegral::new(&Waveform::new(0.6));
        Self {
            aspiration_noise: Noise::new(seed + 1, sample_rate, 500.0),
            phase: 0.0,
            last_integral: waveform.compute(0.0),
            waveform,
            sample_rate,
            wiggle: Wiggle::new(1.0 / sample_rate, 10.0, seed + 2),
        }
    }

    pub fn get_phase(&self) -> f32 {
        self.phase
    }

    pub fn process(
        &mut self,
        frequency: f32,
        tenseness: f32,
        intensity: f32,
        loudness: f32,
        aspiration_level: f32,
    ) -> f32 {
        let noise = self.aspiration_noise.process();

        let d = frequency / self.sample_rate;
        self.phase += d;
        if 1.0 < self.phase {
            self.phase -= 1.0;
            self.waveform = WaveformIntegral::new(&Waveform::new(tenseness));
            self.last_integral = self.waveform.compute(0.0);
        }

        // let out = intensity * loudness * self.waveform.normalized_lf_waveform(self.phase);
        let integral = self.waveform.compute(self.phase);
        let out = intensity * loudness * (integral - self.last_integral) / d;
        self.last_integral = integral;

        let noise = self.get_noise_modulator(tenseness * intensity) * noise;
        let aspiration = intensity
            * (1.0 - tenseness.sqrt())
            * noise
            * (0.2 + 0.01 * self.wiggle.process())
            * aspiration_level;

        out + aspiration
    }

    fn get_noise_modulator(&mut self, rate: f32) -> f32 {
        let voiced = 0.1 + 0.2 * 0.0f32.max((PI * 2.0 * self.phase).sin());
        lerp(0.3, voiced, rate)
    }
}

/// Liljencrants-Fant waveform
struct Waveform {
    alpha: f32,
    e0: f32,
    epsilon: f32,
    shift: f32,
    delta: f32,
    te: f32,
    omega: f32,
}

impl Waveform {
    fn new(tenseness: f32) -> Self {
        let rd = (3.0 * (1.0 - tenseness)).clamp(0.5, 2.7);

        let ra = -0.01 + 0.048 * rd;
        let rk = 0.224 + 0.118 * rd;
        let rg = (rk / 4.0) * (0.5 + 1.2 * rk) / (0.11 * rd - ra * (0.5 + 1.2 * rk));

        let ta = ra;
        let tp = 1.0 / (2.0 * rg);
        let te = tp + tp * rk;

        let epsilon = 1.0 / ta;
        let shift = (-epsilon * (1.0 - te)).exp();
        let delta = 1.0 - shift; //divide by this to scale RHS

        let rhs_integral = ((1.0 / epsilon) * (shift - 1.0) + (1.0 - te) * shift) / delta;

        let total_lower_integral = -(te - tp) / 2.0 + rhs_integral;
        let total_upper_integral = -total_lower_integral;

        let omega = PI / tp;
        let s = (omega * te).sin();
        // need E0*e^(alpha*Te)*s = -1 (to meet the return at -1)
        // and E0*e^(alpha*Tp/2) * Tp*2/pi = totalUpperIntegral
        //             (our approximation of the integral up to Tp)
        // writing x for e^alpha,
        // have E0*x^Te*s = -1 and E0 * x^(Tp/2) * Tp*2/pi = totalUpperIntegral
        // dividing the second by the first,
        // letting y = x^(Tp/2 - Te),
        // y * Tp*2 / (pi*s) = -totalUpperIntegral;
        let y = -PI * s * total_upper_integral / (tp * 2.0);
        let z = y.ln();
        let alpha = z / (tp / 2.0 - te);
        let e0 = -1.0 / (s * (alpha * te).exp());

        Self {
            alpha,
            e0,
            epsilon,
            shift,
            delta,
            te,
            omega,
        }
    }

    #[allow(dead_code)]
    fn compute(&self, t: f32) -> f32 {
        if self.te < t {
            (-(-self.epsilon * (t - self.te)).exp() + self.shift) / self.delta
        } else {
            self.e0 * (self.alpha * t).exp() * (self.omega * t).sin()
        }
    }
}

pub struct WaveformIntegral {
    te: f32,
    e0: f32,
    alpha: f32,
    omega: f32,
    a: f32,
    epsilon: f32,
    b: f32,
    shift: f32,
    c: f32,
    d: f32,
}

impl WaveformIntegral {
    fn new(waveform: &Waveform) -> Self {
        Self {
            te: waveform.te,
            e0: waveform.e0,
            alpha: waveform.alpha,
            omega: waveform.omega,
            a: 1.0 / (waveform.alpha.powi(2) + waveform.omega.powi(2)),
            epsilon: waveform.epsilon,
            b: 1.0 / waveform.epsilon,
            shift: waveform.shift,
            c: 1.0 / waveform.delta,
            d: waveform.e0
                * (waveform.alpha * waveform.te).exp()
                * (waveform.alpha * (waveform.omega * waveform.te).sin()
                    - waveform.omega * (waveform.omega * waveform.te).cos())
                / (waveform.alpha.powi(2) + waveform.omega.powi(2)),
        }
    }

    pub fn compute(&self, t: f32) -> f32 {
        if t <= self.te {
            self.e0
                * (self.alpha * t).exp()
                * (self.alpha * (self.omega * t).sin() - self.omega * (self.omega * t).cos())
                * self.a
        } else {
            (((-self.epsilon * (t - self.te)).exp() - 1.0) * self.b + self.shift * (t - self.te))
                * self.c
                + self.d
        }
    }
}

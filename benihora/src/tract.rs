use std::f64::consts::PI;

use crate::{noise::Noise, IntervalTimer};

use super::{lerp, F};

pub const DEFAULT_TONGUE: (f64, f64) = (12.9, 2.43);

pub struct Tract {
    pub(crate) params: OtherParams,
    pub source: ShapeSource,
    pub current_diameter: Diameter,
    pub target_diameter: Diameter,
    reflections: Reflections,
    pub(crate) new_reflections: Reflections,
    pub state: State,
    pub movement_speed: F, // CM per second
    sample_rate: F,
    update_timer: IntervalTimer,
    fricative_noise: Noise,
    last_obstruction: usize,
    steps_per_process: usize,
    dtime: F,
}

impl Tract {
    pub fn new(steps_per_process: usize, sample_rate: F, seed: u32) -> Self {
        let mouth_length = 44;
        let nose_length = 28;
        let nose_start = mouth_length - nose_length + 1;
        let source = ShapeSource::new(mouth_length, nose_length);
        let mut diameter = Diameter::new(&source);
        let mut reflections = Reflections::new(mouth_length, nose_length);
        source.compute_diameter(&mut diameter);
        diameter.compute_reflections(&mut reflections);

        Tract {
            params: OtherParams::new(nose_start, sample_rate * steps_per_process as f64),
            source,
            current_diameter: diameter.clone(),
            target_diameter: diameter.clone(),
            reflections: reflections.clone(),
            new_reflections: reflections.clone(),
            state: State::new(mouth_length, nose_length),
            movement_speed: 15.0,
            sample_rate,
            update_timer: IntervalTimer::new_overflowed(0.02),
            fricative_noise: Noise::new(seed + 1, sample_rate, 1000.0),
            last_obstruction: usize::MAX,
            steps_per_process,
            dtime: 1.0 / (sample_rate * steps_per_process as f64),
        }
    }

    pub fn process(&mut self, intensity: F, x: F) -> F {
        if self.update_timer.overflowed() {
            self.update_block(self.update_timer.interval);
        }
        let lambda = self.update_timer.progress();
        self.update_timer.update(1.0 / self.sample_rate as f64);

        let fricative_noise = self.fricative_noise.process();

        // Add a bit of noise to avoid subnormal
        let x = x + fricative_noise * 1.0e-16;

        let turbulence_noise = fricative_noise * intensity;
        let mut vocal_out = 0.0;
        for _ in 0..self.steps_per_process {
            let (mouth, nose) = self.run_step(x, turbulence_noise, lambda);
            vocal_out += mouth + nose;
        }

        (vocal_out / self.steps_per_process as f64).into()
    }

    pub fn run_step(&mut self, glottal_output: F, turbulence_noise: F, lambda: F) -> (F, F) {
        self.state.process_transients(self.dtime);
        self.state
            .process_turbulence_noise(self.dtime, turbulence_noise);

        let lip_output = self.state.process_mouth(
            &self.params,
            &self.reflections,
            &self.new_reflections,
            lambda,
            glottal_output,
        );
        let nose_out = self.state.process_nose(
            &self.params,
            &self.new_reflections,
            lerp(
                self.reflections.nose[0],
                self.new_reflections.nose[0],
                lambda,
            ),
        );

        (lip_output, nose_out)
    }

    pub fn update_block(&mut self, block_time: F) {
        self.current_diameter
            .reshape(&self.target_diameter, block_time * self.movement_speed);
        {
            let mut new_last_obstruction = usize::MAX; // indicates whether it is an occlusion
            for (i, d) in self.current_diameter.mouth.iter().enumerate() {
                if *d <= 0.0 {
                    new_last_obstruction = i;
                }
            }
            if self.last_obstruction != usize::MAX
                && new_last_obstruction == usize::MAX
                && self.current_diameter.nose[0].powi(2) < 0.05
            {
                self.state.transients.push(Transient {
                    position: self.last_obstruction,
                    delay: 0.02, // The original code had this as the same as the block time
                    time_alive: 0.0,
                    strength: 0.3,
                });
            }
            self.last_obstruction = new_last_obstruction;
        }

        std::mem::swap(&mut self.reflections, &mut self.new_reflections);
        self.current_diameter
            .compute_reflections(&mut self.new_reflections);
    }

    pub fn update_diameter(&mut self) {
        self.source.compute_diameter(&mut self.target_diameter);

        self.state.turbulences.iter_mut().for_each(|t| t.on = false);
        for constriction in &self.source.other_constrictions {
            let diameter_range = 0.3..0.7;
            if !(1.0..(self.source.length - 1) as F).contains(&constriction.0)
                || !diameter_range.contains(&constriction.1)
            {
                continue;
            }
            if let Some(t) = self
                .state
                .turbulences
                .iter_mut()
                .find(|t| t.index == constriction.0 && t.diameter == constriction.1)
            {
                t.on = true;
            } else {
                self.state
                    .turbulences
                    .push(Turbulence::new(constriction.0, constriction.1));
            }
        }
    }

    /// value: 0.01 - 0.4
    pub fn set_velum_target(&mut self, velum_target: F) {
        self.target_diameter.nose[0] = velum_target;
    }
}

pub struct ShapeSource {
    pub length: usize,
    pub nose_length: usize,
    pub blade_start: usize,
    pub tip_start: usize,
    pub lip_start: usize,
    pub nose_start: usize,

    original_diameter: Vec<F>,

    pub tongue: (F, F), // (index, diameter) // TODO index -> rate, should this be here?
    pub other_constrictions: Vec<(F, F)>,
}

impl ShapeSource {
    pub fn new(length: usize, nose_length: usize) -> Self {
        let original_diameter = (0..length)
            .map(|i| {
                if (i as f64) < (7.0 / 44.0 * length as F - 0.5) {
                    0.6
                } else if (i as f64) < (12.0 / 44.0 * length as F) {
                    1.1
                } else {
                    1.5
                }
            })
            .collect();

        ShapeSource {
            length,
            nose_length,
            blade_start: (10.0 / 44.0 * length as f32).floor() as usize,
            tip_start: (32.0 / 44.0 * length as f32).floor() as usize,
            lip_start: (39.0 / 44.0 * length as f32).floor() as usize,
            nose_start: length - nose_length + 1,
            original_diameter,
            tongue: DEFAULT_TONGUE,
            other_constrictions: Vec::new(),
        }
    }

    pub fn compute_diameter(&self, diameter: &mut Diameter) {
        const GRID_OFFSET: F = 1.7;

        let (tongue_index, tongue_diameter) = self.tongue;

        diameter.mouth.copy_from_slice(&self.original_diameter);
        for i in self.blade_start..self.lip_start {
            let t = 1.1 * PI * (tongue_index - i as F) / (self.tip_start - self.blade_start) as F;
            let fixed_tongue_diameter = 2.0 + (tongue_diameter - 2.0) / 1.5;
            let mut curve = (1.5 - fixed_tongue_diameter + GRID_OFFSET) * t.cos();
            if i == self.blade_start - 2 || i == self.lip_start - 1 {
                curve *= 0.8;
            }
            if i == self.blade_start || i == self.lip_start - 2 {
                curve *= 0.94;
            }
            diameter.mouth[i] = 1.5 - curve;
        }

        for constriction in self.other_constrictions.iter() {
            let index = constriction.0;
            let mut d = constriction.1;
            d = (d - 0.3).max(0.0);

            let width = if index < 25.0 {
                10.0
            } else if index >= self.tip_start as F {
                5.0
            } else {
                10.0 - 5.0 * (index - 25.0) / (self.tip_start as F - 25.0)
            };

            if index >= 2.0 && index < self.length as F && d < 3.0 {
                // && y<tractCanvas.height
                let int_index = index.round() as isize;
                for i in -width.ceil() as isize - 1..width as isize + 1 {
                    let idx = int_index + i;

                    if idx < 0 || idx >= self.length as isize {
                        continue;
                    }
                    let idx = idx as usize;
                    let relpos = (idx as F - index).abs() - 0.5;
                    let shrink = if relpos <= 0.0 {
                        0.0
                    } else if relpos > width {
                        1.0
                    } else {
                        0.5 * (1.0 - (PI * relpos / width).cos())
                    };
                    if d < diameter.mouth[idx] {
                        diameter.mouth[idx] = d + (diameter.mouth[idx] - d) * shrink;
                    }
                }
            }
        }
    }

    pub fn tongue_clamp(&self, index: F, diameter: F) -> (F, F) {
        const INNER_RADIUS: F = 2.05;
        const OUTER_RADIUS: F = 3.5;
        let lower_index_bound = self.blade_start as F + 2.0;
        let upper_index_bound = self.tip_start as F - 3.0;
        let index_center = (lower_index_bound + upper_index_bound) * 0.5;

        let mut from_point = (OUTER_RADIUS - diameter) / (OUTER_RADIUS - INNER_RADIUS);
        from_point = from_point.clamp(0.0, 1.0);
        from_point = from_point.powf(0.58) - 0.2 * (from_point.powi(2) - from_point); // horrible kludge to fit curve to straight line
        let out = from_point * 0.5 * (upper_index_bound - lower_index_bound);
        let index = index.clamp(index_center - out, index_center + out);

        let diameter = diameter.clamp(INNER_RADIUS, OUTER_RADIUS);

        (index, diameter)
    }
}

#[derive(Clone)]
pub struct Diameter {
    nose_start: usize,
    tip_start: usize,
    pub mouth: Vec<F>,
    pub nose: Vec<F>,
}

impl Diameter {
    pub fn new(source: &ShapeSource) -> Self {
        let mut nose: Vec<_> = (0..source.nose_length)
            .map(|i| {
                let d = 2.0 * i as F / source.nose_length as F;
                (1.9 as F).min(if d < 1.0 {
                    0.4 + 1.6 * d
                } else {
                    0.5 + 1.5 * (2.0 - d)
                })
            })
            .collect();
        nose[0] = 0.01; // velum

        Diameter {
            nose_start: source.nose_start,
            tip_start: source.tip_start,
            mouth: vec![0.0; source.length],
            nose,
        }
    }

    pub fn reshape(&mut self, target_diameter: &Diameter, amount: F) {
        for i in 0..self.mouth.len() {
            let slow_return = if i < self.nose_start {
                0.6
            } else if i >= self.tip_start {
                1.0
            } else {
                0.6 + 0.4 * (i as F - self.nose_start as F)
                    / (self.tip_start as F - self.nose_start as F)
            };
            self.mouth[i] = move_towards(
                self.mouth[i],
                target_diameter.mouth[i],
                slow_return * amount,
                2.0 * amount,
            );
        }

        // velum
        self.nose[0] = move_towards(
            self.nose[0],
            target_diameter.nose[0],
            0.25 * amount,
            0.1 * amount,
        );
    }

    pub fn compute_reflections(&mut self, reflections: &mut Reflections) {
        let area: Vec<_> = self.mouth.iter().map(|d| d * d).collect();
        for i in 0..self.mouth.len() - 1 {
            reflections.mouth[i] = if area[i + 1] == 0.0 {
                0.999
            } else {
                (area[i] - area[i + 1]) / (area[i] + area[i + 1])
            };
        }

        let nose_area: Vec<_> = self.nose.iter().map(|d| d * d).collect();
        for i in 0..self.nose.len() - 1 {
            reflections.nose[i] =
                (nose_area[i] - nose_area[i + 1]) / (nose_area[i] + nose_area[i + 1]);
        }

        let sum = area[self.nose_start] + area[self.nose_start + 1] + nose_area[0];
        reflections.junction_left = 2.0 * area[self.nose_start] / sum - 1.0;
        reflections.junction_right = 2.0 * area[self.nose_start + 1] / sum - 1.0;
        reflections.junction_nose = 2.0 * nose_area[0] / sum - 1.0;
    }
}

#[derive(Clone)]
pub struct Reflections {
    mouth: Vec<F>,
    pub(crate) nose: Vec<F>,

    junction_left: F,
    junction_right: F,
    junction_nose: F,
}

impl Reflections {
    pub fn new(length: usize, nose_length: usize) -> Self {
        Reflections {
            mouth: vec![0.0; length - 1],
            nose: vec![0.0; nose_length - 1],

            junction_left: 0.0,
            junction_right: 0.0,
            junction_nose: 0.0,
        }
    }
}

pub struct State {
    r: Vec<F>,
    l: Vec<F>,
    r_: Vec<F>,
    l_: Vec<F>,

    nose_r: Vec<F>,
    nose_l: Vec<F>,
    nose_r_: Vec<F>,
    nose_l_: Vec<F>,

    transients: Vec<Transient>,
    turbulences: Vec<Turbulence>,
}

impl State {
    pub fn new(length: usize, nose_length: usize) -> Self {
        State {
            r: vec![0.0; length],
            l: vec![0.0; length],
            r_: vec![0.0; length],
            l_: vec![0.0; length],

            nose_r: vec![0.0; nose_length],
            nose_l: vec![0.0; nose_length],
            nose_r_: vec![0.0; nose_length],
            nose_l_: vec![0.0; nose_length],

            transients: Vec::new(),
            turbulences: Vec::new(),
        }
    }

    pub fn process_transients(&mut self, dtime: F) {
        for trans in self.transients.iter_mut() {
            if trans.delay > 0.0 {
                trans.delay -= dtime;
                continue;
            }
            let amplitude = trans.strength * (2 as F).powf(-Transient::EXPONENT * trans.time_alive);
            self.r[trans.position] += amplitude * 0.5;
            self.l[trans.position] += amplitude * 0.5;
            trans.time_alive += dtime;
        }

        const LIFE_TIME: F = 0.2;
        self.transients.retain(|t| t.time_alive <= LIFE_TIME)
    }

    pub fn process_turbulence_noise(&mut self, dtime: f64, turbulence_noise: F) {
        let mut turbulences = Vec::new();
        std::mem::swap(&mut turbulences, &mut self.turbulences);
        for turbulence in &mut turbulences {
            turbulence.update_intensity(dtime);
            let amplitude = turbulence.strength * turbulence.intensity;
            if amplitude == 0.0 {
                continue;
            }

            // turbulence noise appears a little ahead
            self.add_noise_at_index(turbulence.index + 1.0, turbulence_noise * amplitude);
        }
        turbulences.retain(|t| t.on || t.intensity > 0.0);
        std::mem::swap(&mut turbulences, &mut self.turbulences);
    }

    pub fn process_mouth(
        &mut self,
        params: &OtherParams,
        reflections: &Reflections,
        new_reflections: &Reflections,
        lambda: F,
        glottal_output: F,
    ) -> F {
        let length = self.r.len();

        //self.glottalReflection = -0.8 + 1.6 * Glottis.newTenseness;
        self.r_[0] = self.l[0] * params.glottal_reflection + glottal_output;
        self.l_[length - 1] = self.r[length - 1] * params.lip_reflection;

        for i in 0..length - 1 {
            let r = lerp(reflections.mouth[i], new_reflections.mouth[i], lambda);
            let w = r * (self.r[i] + self.l[i + 1]);
            self.r_[i + 1] = self.r[i] - w;
            self.l_[i] = self.l[i + 1] + w;
        }

        // junction with nose
        let i = params.nose_start;
        let r = lerp(
            reflections.junction_left,
            new_reflections.junction_left,
            lambda,
        );
        self.l_[i - 1] = r * self.r[i - 1] + (1.0 + r) * (self.nose_l[0] + self.l[i]);
        let r = lerp(
            reflections.junction_right,
            new_reflections.junction_right,
            lambda,
        );
        self.r_[i] = r * self.l[i] + (1.0 + r) * (self.r[i - 1] + self.nose_l[0]);
        let r = lerp(
            reflections.junction_nose,
            new_reflections.junction_nose,
            lambda,
        );
        self.nose_r_[0] = r * self.nose_l[0] + (1.0 + r) * (self.l[i] + self.r[i - 1]);

        for i in 0..length {
            self.r[i] = (self.r_[i] * params.fade).clamp(-1.0, 1.0);
            self.l[i] = (self.l_[i] * params.fade).clamp(-1.0, 1.0);
        }

        self.r[length - 1]
    }

    pub fn process_nose(&mut self, params: &OtherParams, reflections: &Reflections, first: F) -> F {
        let length: usize = self.nose_r.len();
        self.nose_l_[length - 1] = self.nose_r[length - 1] * params.lip_reflection;

        let w = first * (self.nose_r[0] + self.nose_l[1]);
        self.nose_r_[1] = self.nose_r[0] - w;
        self.nose_l_[0] = self.nose_l[1] + w;

        for i in 1..length - 1 {
            let w = reflections.nose[i] * (self.nose_r[i] + self.nose_l[i + 1]);
            self.nose_r_[i + 1] = self.nose_r[i] - w;
            self.nose_l_[i] = self.nose_l[i + 1] + w;
        }

        for i in 0..length {
            self.nose_r[i] = (self.nose_r_[i] * params.fade).clamp(-1.0, 1.0);
            self.nose_l[i] = (self.nose_l_[i] * params.fade).clamp(-1.0, 1.0);
        }

        self.nose_r[length - 1]
    }

    fn add_noise_at_index(&mut self, index: F, noise: F) {
        let i = index.floor() as usize;
        let delta = index - i as F;

        let noise0 = noise * (1.0 - delta);
        let noise1 = noise * delta;
        self.r[i] += noise0 * 0.5;
        self.l[i] += noise0 * 0.5;
        self.r[i + 1] += noise1 * 0.5;
        self.l[i + 1] += noise1 * 0.5;
    }
}

pub struct OtherParams {
    nose_start: usize,
    glottal_reflection: F,
    lip_reflection: F,
    fade: F,
}

impl OtherParams {
    pub fn new(nose_start: usize, sample_rate: F) -> Self {
        OtherParams {
            nose_start,
            glottal_reflection: 0.75,
            lip_reflection: -0.85,
            fade: 0.999f64.powf(96000.0 / sample_rate),
        }
    }
}

fn move_towards(current: F, target: F, up: F, down: F) -> F {
    if current < target {
        target.min(current + up)
    } else {
        target.max(current - down)
    }
}

struct Transient {
    position: usize,
    delay: F,
    time_alive: F,
    strength: F,
}

impl Transient {
    const EXPONENT: F = 200.0;
}

#[derive(Clone)]
struct Turbulence {
    index: F,
    diameter: F,
    strength: F,
    intensity: F,
    on: bool,
}

impl Turbulence {
    fn new(index: F, diameter: F) -> Self {
        let thinness = (8.0 * (0.7 - diameter)).clamp(0.0, 1.0);
        let openness = (30.0 * (diameter - 0.3)).clamp(0.0, 1.0);
        let strength = 0.66 * thinness * openness;
        Self {
            index,
            diameter,
            strength,
            intensity: 0.0,
            on: true,
        }
    }

    fn update_intensity(&mut self, dtime: f64) {
        let attack_time = 0.1;
        if self.on {
            self.intensity = (self.intensity + dtime / attack_time).min(1.0);
        } else {
            self.intensity = (self.intensity - dtime / attack_time).max(0.0);
        }
    }
}

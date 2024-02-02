use benihora_vst_ui;

#[derive(serde::Deserialize, serde::Serialize)]
pub(crate) struct FloatParam {
    pub name: String,
    pub value: f32,
    pub normalized_value: f32,
    pub default: f32,
    pub range: FloatRange,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub(crate) enum FloatRange {
    Linear { min: f32, max: f32 },
    Skewed { min: f32, max: f32, factor: f32 },
}

impl FloatParam {
    pub fn new(name: &'static str, value: f32, range: FloatRange) -> Self {
        Self {
            name: name.to_owned(),
            value,
            normalized_value: range.normalize(value),
            default: value,
            range,
        }
    }

    pub fn smoothed_next(&mut self) -> f32 {
        self.value
    }
}

impl benihora_vst_ui::ui::Param for FloatParam {
    fn set(&mut self, value: f32) {
        self.value = value;
        self.normalized_value = self.range.normalize(value);
    }

    fn modulated_normalized_value(&self) -> f32 {
        self.normalized_value
    }

    fn default_plain_value(&self) -> f32 {
        self.default
    }

    fn preview_plain(&self, normalized: f32) -> f32 {
        self.range.unnormalize(normalized)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn to_string(&self) -> String {
        format!("{:.3}", self.value)
    }
}

impl FloatRange {
    pub fn normalize(&self, value: f32) -> f32 {
        match self {
            Self::Linear { min, max } => (value.clamp(*min, *max) - min) / (max - min),
            Self::Skewed { min, max, factor } => {
                ((value.clamp(*min, *max) - min) / (max - min)).powf(*factor)
            }
        }
    }

    pub fn unnormalize(&self, normalized: f32) -> f32 {
        match self {
            Self::Linear { min, max } => normalized * (max - min) + min,
            Self::Skewed { min, max, factor } => {
                (normalized.powf(factor.recip()) * (max - min)) + min
            }
        }
    }

    pub fn gain_skew_factor(min_db: f32, max_db: f32) -> f32 {
        debug_assert!(min_db < max_db);

        let min_gain = db_to_gain(min_db);
        let max_gain = db_to_gain(max_db);
        let middle_db = (max_db + min_db) / 2.0;
        let middle_gain = db_to_gain(middle_db);

        0.5f32.log((middle_gain - min_gain) / (max_gain - min_gain))
    }
}

#[inline]
pub fn db_to_gain(dbs: f32) -> f32 {
    if dbs > -100.0 {
        10.0f32.powf(dbs * 0.05)
    } else {
        0.0
    }
}

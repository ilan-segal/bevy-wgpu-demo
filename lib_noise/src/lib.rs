use std::num::NonZero;

use noise::{NoiseFn, ScalePoint, Simplex, TranslatePoint};

type ScaledTranslatedNoise = TranslatePoint<ScalePoint<Simplex>>;

struct FractalNoisePart {
    a: f64,
    noise: ScaledTranslatedNoise,
}

impl<const DIM: usize> NoiseFn<f64, DIM> for FractalNoisePart
where
    ScaledTranslatedNoise: NoiseFn<f64, DIM>,
{
    fn get(&self, point: [f64; DIM]) -> f64 {
        self.noise.get(point) * self.a
    }
}

pub struct FractalNoise {
    inverse_of_sum_of_scales: f64,
    parts: Vec<FractalNoisePart>,
}

impl FractalNoise {
    pub fn new(seed: u32, layers: NonZero<u32>, noise_scale: f64) -> Self {
        let layers = layers.get();
        let sum_of_layer_scales = 1.0 - 0.5_f64.powi(layers as i32);
        let inverse_of_sum_of_scales = sum_of_layer_scales.recip();
        let parts = (0..layers)
            .map(|k| {
                let seed = seed.rotate_left(k);
                let a = 0.5_f64.powi(k as i32);
                let scale = noise_scale * a;
                let translation = 0.5 * scale;
                let simplex = Simplex::new(seed);
                let scaled = ScalePoint::new(simplex).set_scale(scale);
                let translated = TranslatePoint::new(scaled).set_translation(translation);
                let part = FractalNoisePart {
                    a,
                    noise: translated,
                };
                return part;
            })
            .collect();
        return Self {
            inverse_of_sum_of_scales,
            parts,
        };
    }
}

impl<const DIM: usize> NoiseFn<f64, DIM> for FractalNoise
where
    FractalNoisePart: NoiseFn<f64, DIM>,
{
    fn get(&self, point: [f64; DIM]) -> f64 {
        self.parts.iter().map(|noise| noise.get(point)).sum::<f64>() * self.inverse_of_sum_of_scales
    }
}

impl<const DIM: usize> NoiseFn<i32, DIM> for FractalNoise
where
    FractalNoise: NoiseFn<f64, DIM>,
{
    fn get(&self, point: [i32; DIM]) -> f64 {
        self.get(point.map(|x| x as f64))
    }
}

use rustfft::{num_complex::Complex, Fft, FftPlanner};
use std::sync::Arc;

const FFT_SIZE: usize = 2048;
const BAND_COUNT: usize = 20;

/// Windowed FFT analyzer for the audio island's 20-band spectrum.
pub struct SpectrumAnalyzer {
    fft: Arc<dyn Fft<f32>>,
    sample_rate: u32,
    hann: Vec<f32>,
    band_edges: [(usize, usize); BAND_COUNT],
    smoothed: [f32; BAND_COUNT],
    input: Vec<Complex<f32>>,
    magnitudes: Vec<f32>,
}

impl SpectrumAnalyzer {
    pub fn new(sample_rate: u32) -> Self {
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);
        let hann = (0..FFT_SIZE)
            .map(|i| {
                0.5 - 0.5
                    * (std::f32::consts::TAU * i as f32 / (FFT_SIZE - 1) as f32).cos()
            })
            .collect();
        let band_edges = make_band_edges(sample_rate.max(1));

        Self {
            fft,
            sample_rate: sample_rate.max(1),
            hann,
            band_edges,
            smoothed: [0.0; BAND_COUNT],
            input: vec![Complex::new(0.0, 0.0); FFT_SIZE],
            magnitudes: vec![0.0; FFT_SIZE / 2 + 1],
        }
    }

    pub fn analyze(&mut self, mono: &[f32]) -> ([f32; BAND_COUNT], [f32; 4]) {
        self.input.fill(Complex::new(0.0, 0.0));
        let take = mono.len().min(FFT_SIZE);
        let source = &mono[mono.len().saturating_sub(FFT_SIZE)..];
        let offset = FFT_SIZE - take;
        for (i, sample) in source.iter().enumerate() {
            self.input[offset + i].re = sample * self.hann[offset + i];
        }

        self.fft.process(&mut self.input);
        for (i, magnitude) in self.magnitudes.iter_mut().enumerate() {
            *magnitude = self.input[i].norm();
        }

        for (i, &(start, end)) in self.band_edges.iter().enumerate() {
            let (sum, count) = self.magnitudes[start..end]
                .iter()
                .fold((0.0f32, 0usize), |(sum, count), value| {
                    (sum + value, count + 1)
                });
            let magnitude = sum / count.max(1) as f32;
            let db = 20.0 * (magnitude / FFT_SIZE as f32 + 1e-9).log10();
            let target = ((db + 80.0) / 70.0).clamp(0.0, 1.0);
            let coefficient = if target > self.smoothed[i] { 0.6 } else { 0.15 };
            self.smoothed[i] += (target - self.smoothed[i]) * coefficient;
        }

        let compact = [
            max_in_range(&self.smoothed, &self.band_edges, self.sample_rate, 40.0, 150.0),
            max_in_range(&self.smoothed, &self.band_edges, self.sample_rate, 150.0, 600.0),
            max_in_range(&self.smoothed, &self.band_edges, self.sample_rate, 600.0, 2500.0),
            max_in_range(&self.smoothed, &self.band_edges, self.sample_rate, 2500.0, 10000.0),
        ];

        (self.smoothed, compact)
    }
}

fn make_band_edges(sample_rate: u32) -> [(usize, usize); BAND_COUNT] {
    let max_bin = FFT_SIZE / 2;
    let nyquist = sample_rate as f32 / 2.0;
    let upper_hz = 16_000.0f32.min(nyquist).max(40.0);
    let lower_hz = 40.0f32.min(upper_hz);
    let ratio = upper_hz / lower_hz;
    let mut edges = [(0usize, 1usize); BAND_COUNT];
    let mut previous = 1usize;

    for (i, edge) in edges.iter_mut().enumerate() {
        let low_hz = lower_hz * ratio.powf(i as f32 / BAND_COUNT as f32);
        let high_hz = lower_hz * ratio.powf((i + 1) as f32 / BAND_COUNT as f32);
        let mut start = (low_hz * FFT_SIZE as f32 / sample_rate as f32).floor() as usize;
        let mut end = (high_hz * FFT_SIZE as f32 / sample_rate as f32).ceil() as usize;
        start = start.clamp(previous, max_bin.saturating_sub(1));
        end = end.clamp(start + 1, max_bin + 1);
        *edge = (start, end);
        previous = end.min(max_bin);
    }
    edges
}

fn max_in_range(
    values: &[f32; BAND_COUNT],
    edges: &[(usize, usize); BAND_COUNT],
    sample_rate: u32,
    low_hz: f32,
    high_hz: f32,
) -> f32 {
    values
        .iter()
        .zip(edges)
        .filter_map(|(&value, &(start, end))| {
            let center = (start + end) as f32 * 0.5 * sample_rate as f32 / FFT_SIZE as f32;
            (center >= low_hz && center < high_hz).then_some(value)
        })
        .fold(0.0, f32::max)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(frequency: f32, sample_rate: u32) -> Vec<f32> {
        (0..FFT_SIZE)
            .map(|i| {
                (std::f32::consts::TAU * frequency * i as f32 / sample_rate as f32).sin()
            })
            .collect()
    }

    #[test]
    fn sixty_hertz_sine_peaks_in_low_bands() {
        let mut analyzer = SpectrumAnalyzer::new(48_000);
        let (bands, _) = analyzer.analyze(&sine(60.0, 48_000));
        let peak = bands.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).unwrap().0;
        assert!((0..=3).contains(&peak), "peak band was {peak}");
    }

    #[test]
    fn five_kilohertz_sine_peaks_in_high_bands() {
        let mut analyzer = SpectrumAnalyzer::new(48_000);
        let (bands, _) = analyzer.analyze(&sine(5_000.0, 48_000));
        let peak = bands.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).unwrap().0;
        assert!(peak >= 14, "peak band was {peak}");
    }

    #[test]
    fn zero_signal_produces_zero_bars() {
        let mut analyzer = SpectrumAnalyzer::new(48_000);
        let (bands, compact) = analyzer.analyze(&[0.0; FFT_SIZE]);
        assert!(bands.iter().all(|value| *value == 0.0));
        assert!(compact.iter().all(|value| *value == 0.0));
    }

    #[test]
    fn one_kilohertz_sine_peaks_in_third_compact_bar() {
        let mut analyzer = SpectrumAnalyzer::new(48_000);
        let (_, compact) = analyzer.analyze(&sine(1_000.0, 48_000));
        let peak = compact
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .unwrap()
            .0;
        assert_eq!(peak, 2);
    }
}

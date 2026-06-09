use rustfft::{FftPlanner, num_complex::Complex};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;

const MFCC_BINS: usize = 13;
const MEL_FILTERS: usize = 40;
const FRAME_SIZE: usize = 400; // 25ms at 16kHz
const FRAME_STRIDE: usize = 160; // 10ms at 16kHz

pub struct WakewordEngine {
    templates: Vec<Vec<Vec<f32>>>,
    templates_path: PathBuf,
    sensitivity: f32,
}

impl WakewordEngine {
    pub fn new(templates_path: PathBuf, sensitivity: f32) -> Self {
        let mut engine = Self {
            templates: Vec::new(),
            templates_path,
            sensitivity,
        };
        let _ = engine.load_templates();
        engine
    }

    pub fn has_templates(&self) -> bool {
        !self.templates.is_empty()
    }

    pub fn add_template(&mut self, audio: &[f32]) {
        let mfccs = extract_mfcc(audio);
        self.templates.push(mfccs);
    }

    pub fn save_templates(&self) -> Result<(), String> {
        let serialized = serde_json::to_string(&self.templates).map_err(|e| e.to_string())?;
        if let Some(parent) = self.templates_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut file = File::create(&self.templates_path).map_err(|e| e.to_string())?;
        file.write_all(serialized.as_bytes())
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn load_templates(&mut self) -> Result<(), String> {
        if !self.templates_path.exists() {
            return Err("Templates file not found".to_string());
        }
        let mut file = File::open(&self.templates_path).map_err(|e| e.to_string())?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|e| e.to_string())?;
        self.templates = serde_json::from_str(&contents).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn clear_templates(&mut self) {
        self.templates.clear();
    }

    pub fn remove_template(&mut self, index: usize) {
        if index < self.templates.len() {
            self.templates.remove(index);
        }
    }

    pub fn detect(&self, audio: &[f32]) -> bool {
        if self.templates.is_empty() {
            return false;
        }

        // Basic silence detection: if audio is too quiet, it's not a wake word.
        let mut sum_squares = 0.0;
        for &sample in audio {
            sum_squares += sample * sample;
        }
        let rms = (sum_squares / audio.len() as f32).sqrt();
        if rms < 0.01 {
            // Magic threshold for absolute silence
            return false;
        }

        let input_mfcc = extract_mfcc(audio);
        if input_mfcc.is_empty() {
            return false;
        }

        let mut min_distance = f32::MAX;
        for template in &self.templates {
            if template.is_empty() {
                continue;
            }
            let dist = dtw(&input_mfcc, template);

            // Normalize distance by length of template to make sensitivity threshold robust
            let normalized_dist = dist / template.len() as f32;

            if normalized_dist < min_distance {
                min_distance = normalized_dist;
            }
        }

        // We convert `sensitivity` (0.0 to 1.0) to a distance threshold.
        // E.g., higher sensitivity = lower threshold = requires closer match.
        // Note: sensitivity usually goes the other way (higher = easier to trigger),
        // so we invert the threshold logic.
        let threshold = (1.0 - self.sensitivity.clamp(0.0, 0.99)) * 50.0; // Scaled down to prevent false positives

        println!(
            "Wakeword engine evaluated audio: dist = {:.2}, threshold = {:.2}",
            min_distance, threshold
        );

        min_distance < threshold
    }
}

// Basic math implementations

fn extract_mfcc(audio: &[f32]) -> Vec<Vec<f32>> {
    if audio.len() < FRAME_SIZE {
        return Vec::new();
    }

    let mut mfccs = Vec::new();
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(FRAME_SIZE);

    let mut start = 0;
    while start + FRAME_SIZE <= audio.len() {
        let frame = &audio[start..start + FRAME_SIZE];

        // Apply Hamming window
        let mut complex_frame: Vec<Complex<f32>> = frame
            .iter()
            .enumerate()
            .map(|(n, &val)| {
                let window = 0.54
                    - 0.46
                        * (2.0 * std::f32::consts::PI * n as f32 / (FRAME_SIZE - 1) as f32).cos();
                Complex {
                    re: val * window,
                    im: 0.0,
                }
            })
            .collect();

        fft.process(&mut complex_frame);

        // Compute power spectrum
        let mut power_spec = Vec::with_capacity(FRAME_SIZE / 2 + 1);
        for c in complex_frame.iter().take(FRAME_SIZE / 2 + 1) {
            let mag: f32 = c.norm();
            // power_spec.push((mag * mag) / FRAME_SIZE as f32);
            power_spec.push(mag);
        }

        // Apply Mel filterbank
        let mel_energies = apply_mel_filterbank(&power_spec);

        // Apply DCT to get MFCCs
        let frame_mfccs = apply_dct(&mel_energies);
        mfccs.push(frame_mfccs);

        start += FRAME_STRIDE;
    }

    mfccs
}

fn apply_mel_filterbank(power_spec: &[f32]) -> Vec<f32> {
    // Highly simplified mel filterbank
    // For a real implementation we would pre-compute filter banks.
    // This distributes energies into MEL_FILTERS bins linearly for simplicity.
    let mut energies = vec![0.0; MEL_FILTERS];
    let bins_per_filter = power_spec.len() / MEL_FILTERS;

    for (i, energy) in energies.iter_mut().enumerate() {
        let start_idx = i * bins_per_filter;
        let end_idx = if i == MEL_FILTERS - 1 {
            power_spec.len()
        } else {
            (i + 1) * bins_per_filter
        };
        let mut sum = 0.0;
        for &val in &power_spec[start_idx..end_idx] {
            sum += val;
        }
        *energy = (sum.max(1e-10)).ln(); // Log energy
    }

    energies
}

fn apply_dct(mel_energies: &[f32]) -> Vec<f32> {
    let mut mfccs = vec![0.0; MFCC_BINS];
    let n = mel_energies.len();
    for (k, mfcc) in mfccs.iter_mut().enumerate() {
        let mut sum = 0.0;
        for (m, &energy) in mel_energies.iter().enumerate() {
            sum += energy * (std::f32::consts::PI * k as f32 * (m as f32 + 0.5) / n as f32).cos();
        }
        *mfcc = sum;
    }
    mfccs
}

fn dtw(seq1: &[Vec<f32>], seq2: &[Vec<f32>]) -> f32 {
    let n = seq1.len();
    let m = seq2.len();

    if n == 0 || m == 0 {
        return f32::MAX;
    }

    let mut dtw = vec![vec![f32::MAX; m + 1]; n + 1];
    dtw[0][0] = 0.0;

    for i in 1..=n {
        for j in 1..=m {
            let cost = euclidean_distance(&seq1[i - 1], &seq2[j - 1]);
            let min_prev = dtw[i - 1][j].min(dtw[i][j - 1]).min(dtw[i - 1][j - 1]);
            dtw[i][j] = cost + min_prev;
        }
    }

    dtw[n][m]
}

fn euclidean_distance(v1: &[f32], v2: &[f32]) -> f32 {
    v1.iter()
        .zip(v2.iter())
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f32>()
        .sqrt()
}

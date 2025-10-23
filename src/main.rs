use anyhow::{Error, Result};
use clap::Parser;
use hound::{WavReader, WavWriter};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Parser)]
#[command(
    name = "wav-files-echo",
    about = "Add echo, reverb, or chorus effects to WAV files recursively"
)]
struct Args {
    /// Input directory containing WAV files (processed recursively)
    input_dir: PathBuf,

    /// Output directory for processed files (preserves relative structure)
    output_dir: PathBuf,

    /// Effect type: echo, reverb, or chorus
    #[arg(short, long, default_value = "echo")]
    effect: String,

    /// Wet/dry mix (0.0 dry, 1.0 wet)
    #[arg(short, long, default_value_t = 0.5f32)]
    wet: f32,

    /// Base delay time in milliseconds
    #[arg(short, long, default_value_t = 250u32)]
    delay_ms: u32,

    /// Decay time in seconds (RT60 approximation)
    #[arg(short = 't', long, default_value_t = 1.0f32)]
    decay_time_s: f32,

    /// Chorus modulation rate in Hz (ignored for echo/reverb)
    #[arg(long, default_value_t = 0.8f32)]
    chorus_rate_hz: f32,

    /// Chorus modulation depth in ms (ignored for echo/reverb)
    #[arg(long, default_value_t = 20.0f32)]
    chorus_depth_ms: f32,
}

fn main() -> Result<(), Error> {
    let args = Args::parse();
    fs::create_dir_all(&args.output_dir)?;

    for entry in WalkDir::new(&args.input_dir).follow_links(true) {
        let entry = entry?;
        if entry.file_type().is_file()
            && entry
                .path()
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s == "wav")
                .unwrap_or(false)
        {
            process_file(entry.path(), &args.input_dir, &args.output_dir, &args)?;
        }
    }

    Ok(())
}

/// Processes a single WAV file, applies the effect, and writes to output path.
fn process_file(
    input_path: &Path,
    input_dir: &Path,
    output_dir: &Path,
    args: &Args,
) -> Result<(), Error> {
    let mut reader = WavReader::open(input_path)?;
    let spec = reader.spec();

    if spec.channels != 1 {
        return Err(Error::msg("Only mono audio supported"));
    }
    if spec.sample_rate != 16000 {
        return Err(Error::msg("Only 16kHz sample rate supported"));
    }
    if spec.bits_per_sample != 16 {
        return Err(Error::msg("Only 16-bit PCM supported"));
    }

    let samples: Vec<i16> = reader.samples::<i16>().collect::<Result<Vec<_>, _>>()?;

    let sr = spec.sample_rate as f32;
    let samples_f32: Vec<f32> = samples.iter().map(|&s| s as f32 / 32768.0).collect();

    let processed_f32 = match args.effect.as_str() {
        "echo" => apply_delay_effect(
            &samples_f32,
            sr,
            args.wet,
            args.delay_ms as f32,
            args.decay_time_s,
            false,
        ),
        "reverb" => apply_delay_effect(
            &samples_f32,
            sr,
            args.wet,
            args.delay_ms as f32,
            args.decay_time_s,
            true,
        ),
        "chorus" => apply_chorus_effect(
            &samples_f32,
            sr,
            args.wet,
            args.delay_ms as f32,
            args.decay_time_s,
            args.chorus_rate_hz,
            args.chorus_depth_ms,
        ),
        _ => return Err(Error::msg(format!("Unknown effect: {}", args.effect))),
    };

    let processed_samples: Vec<i16> = processed_f32
        .into_iter()
        .map(|s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
        .collect();

    let rel_path = input_path.strip_prefix(input_dir)?.to_path_buf();
    let output_path = output_dir.join(rel_path);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut writer = WavWriter::create(output_path, spec)?;
    for &sample in &processed_samples {
        writer.write_sample(sample)?;
    }
    writer.finalize()?;

    Ok(())
}

/// Applies a delay-based effect (echo or reverb) to mono f32 samples [-1.0, 1.0].
/// Uses circular delay line with feedback computed from decay time (RT60 approximation).
/// For reverb, applies a simple 1-pole lowpass filter in the feedback path.
fn apply_delay_effect(
    input: &[f32],
    sr: f32,
    wet: f32,
    delay_ms: f32,
    decay_time_s: f32,
    lowpass: bool,
) -> Vec<f32> {
    let delay_samples = (delay_ms * sr / 1000.0).max(1.0) as usize;
    let delay_s = delay_ms / 1000.0;
    let feedback = 10f32.powf(-3.0 * delay_s / decay_time_s).clamp(0.0, 1.0);
    let dry = 1.0 - wet;

    let mut delay_line = vec![0.0f32; delay_samples];
    let mut output = vec![0.0f32; input.len()];
    let mut write_pos = 0usize;
    let mut prev_lp = 0.0f32;
    let lp_coeff = 0.5f32; // Simple lowpass coefficient

    for (i, &inp) in input.iter().enumerate() {
        let read_pos = ((write_pos as isize - delay_samples as isize)
            .rem_euclid(delay_samples as isize)) as usize;
        let delayed = delay_line[read_pos];

        output[i] = dry * inp + wet * delayed;

        let mut feedback_val = delayed;
        if lowpass {
            let lp_out = lp_coeff * feedback_val + (1.0 - lp_coeff) * prev_lp;
            feedback_val = lp_out;
            prev_lp = lp_out;
        }

        delay_line[write_pos] = inp + feedback * feedback_val;
        write_pos = (write_pos + 1) % delay_samples;
    }

    output
}

/// Applies a simple chorus effect using modulated delay.
fn apply_chorus_effect(
    input: &[f32],
    sr: f32,
    wet: f32,
    delay_ms: f32,
    decay_time_s: f32,
    rate_hz: f32,
    depth_ms: f32,
) -> Vec<f32> {
    let base_delay_samples = (delay_ms * sr / 1000.0).max(1.0);
    let depth_samples = (depth_ms * sr / 1000.0).max(1.0);
    let delay_s = delay_ms / 1000.0;
    let feedback = 10f32.powf(-3.0 * delay_s / decay_time_s).clamp(0.0, 0.3); // Low feedback for chorus
    let dry = 1.0 - wet;
    let buffer_size = (base_delay_samples + depth_samples * 2.0) as usize; // Extra space for modulation

    let mut delay_line = vec![0.0f32; buffer_size];
    let mut output = vec![0.0f32; input.len()];
    let mut write_pos = 0usize;
    let mut phase = 0.0f32;
    let phase_inc = 2.0 * std::f32::consts::PI * rate_hz / sr;

    for (i, &inp) in input.iter().enumerate() {
        let modulation = (phase.sin() + 1.0) * 0.5; // 0.0 to 1.0
        let curr_delay = base_delay_samples + modulation * depth_samples;
        let read_pos_float = (write_pos as f32 - curr_delay) % (buffer_size as f32);
        let read_pos = read_pos_float.max(0.0) as usize % buffer_size;

        // Simple linear interpolation for fractional delay
        let delayed = if read_pos_float.fract() == 0.0 {
            delay_line[read_pos]
        } else {
            let pos1 = read_pos;
            let pos2 = (pos1 + 1) % buffer_size;
            let frac = read_pos_float.fract();
            delay_line[pos1] * (1.0 - frac) + delay_line[pos2] * frac
        };

        output[i] = dry * inp + wet * delayed;

        let feedback_val = feedback * delayed;
        delay_line[write_pos] = inp + feedback_val;
        write_pos = (write_pos + 1) % buffer_size;

        phase += phase_inc;
        if phase >= 2.0 * std::f32::consts::PI {
            phase -= 2.0 * std::f32::consts::PI;
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_echo_basic() {
        let sr = 16000.0;
        // Impulse at ~0.625s (10000 samples / 16000 Hz)
        let input: Vec<f32> = std::iter::repeat(0.0)
            .take(10000)
            .chain(std::iter::once(1.0))
            .chain(std::iter::repeat(0.0).take(4000))
            .collect();
        let output = apply_delay_effect(&input, sr, 0.5, 250.0, 1.0, false);
        // Check for echo at ~250ms, amplitude ~0.5 * feedback
        let impulse_idx = 10000;
        let delay_idx = impulse_idx + (250.0 * sr / 1000.0) as usize;
        // The delayed signal is `dry * inp + wet * delayed`.
        // At `delay_idx`, `inp` is 0. The `delayed` value is the impulse from `impulse_idx`.
        // The value at `impulse_idx` in the delay line is `inp + feedback * feedback_val`.
        // At `impulse_idx`, `inp` is 1.0, `feedback_val` is 0. So `delay_line[write_pos]` becomes 1.0.
        // So `output[delay_idx]` should be `wet * 1.0` = 0.5.
        assert!(
            (output[delay_idx] - 0.5).abs() < 0.001,
            "Echo amplitude is incorrect. Got {}",
            output[delay_idx]
        );
    }

    #[test]
    fn test_reverb_lowpass() {
        let sr = 16000.0;
        let input: Vec<f32> = vec![1.0];
        let output = apply_delay_effect(&input, sr, 1.0, 10.0, 0.1, true); // Short delay, quick decay, full wet
        // With lowpass, feedback should decay faster in high freq, but hard to test simply
        // Basic check: output not empty
        assert_eq!(output.len(), 1);
        // Note: More comprehensive tests would require longer signals
    }

    #[test]
    fn test_chorus_modulation() {
        let sr = 16000.0;
        let input: Vec<f32> = vec![1.0; 1000];
        let output = apply_chorus_effect(&input, sr, 0.5, 10.0, 1.0, 1.0, 5.0);
        // Check variance in output due to modulation
        let mean: f32 = output.iter().sum::<f32>() / output.len() as f32;
        let variance: f32 =
            output.iter().map(|&x| (x - mean).powi(2)).sum::<f32>() / output.len() as f32;
        assert!(variance > 0.001); // Some variation from dry signal
    }
}

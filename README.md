# wav-files-echo

A command-line tool for applying delay-based audio effects (echo, reverb, chorus) to mono WAV files. Processes files recursively from an input directory and outputs results to a specified folder, preserving the original directory structure.

## Features

- **Recursive Processing**: Handles WAV files in subdirectories.
- **Supported Effects**:
  - **Echo**: Simple delay with feedback.
  - **Reverb**: Delay with low-pass filtered feedback for damping.
  - **Chorus**: Modulated delay with sinusoidal LFO for thickening.
- **Format Support**: Optimized for 16-bit mono PCM WAV at 16 kHz (Microsoft WAVE format).
- **DSP Parameters**: Configurable wet/dry mix, delay time, decay time, and chorus-specific modulation.
- **Idiomatic Rust**: Memory-safe, efficient processing with minimal dependencies.

## Installation

### From Source

1. Clone the repository:
   ```bash
   git clone https://github.com/RustedBytes/wav-files-echo.git
   cd wav-files-echo
   ```

2. Build and install:
   ```bash
   cargo install --path .
   ```

## Usage

```bash
wav-files-echo [OPTIONS] <INPUT_DIR> <OUTPUT_DIR>
```

### Required Arguments

- `<INPUT_DIR>`: Path to the directory containing WAV files (processed recursively).
- `<OUTPUT_DIR>`: Path to the output directory for processed files.

### Optional Arguments

- `--effect <EFFECT>`: Effect type (`echo`, `reverb`, or `chorus`). [default: `echo`]
- `-w, --wet <WET>`: Wet/dry mix (0.0 = dry, 1.0 = wet). [default: `0.5`]
- `-d, --delay-ms <DELAY_MS>`: Base delay time in milliseconds. [default: `250`]
- `-t, --decay-time-s <DECAY_TIME_S>`: Decay time in seconds (RT60 approximation). [default: `1.0`]
- `--chorus-rate-hz <CHORUS_RATE_HZ>`: Chorus modulation rate in Hz. [default: `0.8`]
- `--chorus-depth-ms <CHORUS_DEPTH_MS>`: Chorus modulation depth in ms. [default: `20.0`]

Run `wav-files-echo --help` for full details.

## Examples

### Basic Echo Effect

```bash
wav-files-echo ./input ./output --effect echo --wet 0.6 --delay-ms 300
```

Applies a 300ms echo with 60% wet mix to all WAV files in `./input` and saves to `./output`.

### Reverb with Quick Decay

```bash
wav-files-echo ./samples ./reverbed --effect reverb --wet 0.4 --decay-time-s 0.5
```

Creates a short room-like reverb.

### Chorus for Vocal Thickening

```bash
wav-files-echo ./vocals ./chorused --effect chorus --wet 0.3 --chorus-rate-hz 0.5 --chorus-depth-ms 15
```

Adds subtle chorusing to mono vocal tracks.

## Testing

Run the test suite:

```bash
cargo test
```

Includes unit tests for effect implementations using synthetic signals (impulses and steady tones) to verify delay, feedback, and modulation behavior.

## Dependencies

- `clap`: Argument parsing.
- `hound`: WAV file I/O.
- `walkdir`: Recursive directory traversal.

See `Cargo.toml` for versions.

## Contributing

Contributions are welcome! Please:

1. Fork the repository.
2. Create a feature branch (`git checkout -b feature/my-feature`).
3. Commit changes (`git commit -m 'Add my feature'`).
4. Push to the branch (`git push origin feature/my-feature`).
5. Open a Pull Request.

Ensure code passes `cargo fmt`, `cargo clippy`, and `cargo test`.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Cite

```
@software{Smoliakov_Wav_Files_Toolkit,
  author = {Smoliakov, Yehor},
  month = oct,
  title = {{WAV Files Toolkit: A suite of command-line tools for common WAV audio processing tasks, including conversion from other formats, data augmentation, loudness normalization, spectrogram generation, and validation.}},
  url = {https://github.com/RustedBytes/wav-files-toolkit},
  version = {0.4.0},
  year = {2025}
}
```

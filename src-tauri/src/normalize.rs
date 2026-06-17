/// Peak-normalize the WAV file before transcription if the audio is quiet (e.g. whispered).
///
/// Strategy: find the loudest sample; if it's below LOUD_ENOUGH, amplify the whole
/// recording so the peak reaches TARGET. Normal speech is left untouched.
/// Silence (peak < NOISE_FLOOR) is skipped to avoid amplifying empty recordings.
pub fn boost_quiet(path: &std::path::Path) {
    const NOISE_FLOOR: f32 = 0.001; // below this = silence, skip
    const LOUD_ENOUGH: f32 = 0.35; // above this (~−9 dBFS) = normal speech, skip
    const TARGET: f32 = 0.90; // normalize quiet audio to 90 % of full scale

    let reader = match hound::WavReader::open(path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[wispr] normalize: open failed: {e}");
            return;
        }
    };

    let spec = reader.spec();

    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => {
            match reader.into_samples::<f32>().collect::<Result<Vec<_>, _>>() {
                Ok(s) => s,
                Err(_) => return,
            }
        }
        hound::SampleFormat::Int if spec.bits_per_sample == 16 => {
            match reader.into_samples::<i16>().collect::<Result<Vec<_>, _>>() {
                Ok(s) => s
                    .into_iter()
                    .map(|sample| sample as f32 / i16::MAX as f32)
                    .collect(),
                Err(_) => return,
            }
        }
        _ => return,
    };

    if samples.is_empty() {
        return;
    }

    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

    if peak < NOISE_FLOOR || peak >= LOUD_ENOUGH {
        return; // silence or already loud enough
    }

    let gain = TARGET / peak;
    println!("[wispr] normalize: peak={peak:.4} gain={gain:.2}x (whispering detected)");

    let boosted: Vec<f32> = samples
        .iter()
        .map(|&s| (s * gain).clamp(-1.0, 1.0))
        .collect();

    match hound::WavWriter::create(path, spec) {
        Ok(mut writer) => {
            match spec.sample_format {
                hound::SampleFormat::Float => {
                    for s in boosted {
                        let _ = writer.write_sample(s);
                    }
                }
                hound::SampleFormat::Int if spec.bits_per_sample == 16 => {
                    for s in boosted {
                        let sample = (s.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16;
                        let _ = writer.write_sample(sample);
                    }
                }
                _ => return,
            }
            let _ = writer.finalize();
        }
        Err(e) => eprintln!("[wispr] normalize: write failed: {e}"),
    }
}

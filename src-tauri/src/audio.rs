use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;
use std::sync::{Arc, Mutex};

pub struct AudioRecorder {
    samples: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    channels: u16,
    stream: Option<cpal::Stream>,
}

// cpal::Stream on Windows holds COM raw pointers that aren't Send by default,
// but streams are designed to be dropped from any thread.
// Access is always serialised through Mutex<AudioRecorder>.
unsafe impl Send for AudioRecorder {}

impl AudioRecorder {
    // Validates a microphone is available; fails fast at startup.
    pub fn new() -> Result<Self, String> {
        cpal::default_host()
            .default_input_device()
            .ok_or_else(|| "No microphone found".to_string())?;
        Ok(Self {
            samples: Arc::new(Mutex::new(Vec::new())),
            sample_rate: 0,
            channels: 0,
            stream: None,
        })
    }

    pub fn start(&mut self) -> Result<(), String> {
        if self.stream.is_some() {
            return Ok(());
        }

        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| "No microphone found".to_string())?;
        let config = device.default_input_config().map_err(|e| e.to_string())?;

        self.sample_rate = config.sample_rate().0;
        self.channels = config.channels();
        self.samples.lock().unwrap().clear();

        let channels = self.channels as usize;
        let samples = Arc::clone(&self.samples);

        let stream = match config.sample_format() {
            SampleFormat::F32 => build_stream::<f32, _>(&device, &config, samples, channels, |s| s),
            SampleFormat::I16 => build_stream::<i16, _>(&device, &config, samples, channels, |s| s as f32 / 32768.0),
            SampleFormat::U16 => build_stream::<u16, _>(&device, &config, samples, channels, |s| (s as f32 / 32768.0) - 1.0),
            fmt => return Err(format!("Unsupported sample format: {fmt:?}")),
        }?;

        stream.play().map_err(|e| e.to_string())?;
        self.stream = Some(stream);
        Ok(())
    }

    pub fn stop_and_save(&mut self, path: &std::path::Path) -> Result<(), String> {
        self.stream = None; // dropping the stream stops capture

        let samples = self.samples.lock().unwrap();
        if samples.is_empty() {
            return Err("No audio captured".to_string());
        }

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: self.sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };

        let mut writer = hound::WavWriter::create(path, spec).map_err(|e| e.to_string())?;
        for &s in samples.iter() {
            writer.write_sample(s).map_err(|e| e.to_string())?;
        }
        writer.finalize().map_err(|e| e.to_string())
    }

    pub fn is_recording(&self) -> bool {
        self.stream.is_some()
    }
}

fn build_stream<S, F>(
    device: &cpal::Device,
    config: &cpal::SupportedStreamConfig,
    samples: Arc<Mutex<Vec<f32>>>,
    channels: usize,
    convert: F,
) -> Result<cpal::Stream, String>
where
    S: cpal::SizedSample,
    F: Fn(S) -> f32 + Send + 'static,
{
    device
        .build_input_stream(
            &config.clone().into(),
            move |data: &[S], _: &_| {
                let mono: Vec<f32> = if channels == 1 {
                    data.iter().map(|&s| convert(s)).collect()
                } else {
                    data.chunks(channels)
                        .map(|frame| frame.iter().map(|&s| convert(s)).sum::<f32>() / channels as f32)
                        .collect()
                };
                samples.lock().unwrap().extend(mono);
            },
            |e| eprintln!("Audio error: {e}"),
            None,
        )
        .map_err(|e| e.to_string())
}

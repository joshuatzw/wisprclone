use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, serde::Serialize)]
pub struct AudioDeviceInfo {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CaptureInfo {
    pub device_name: String,
    pub sample_rate: u32,
    pub input_channels: u16,
    pub samples: usize,
    pub duration_secs: f32,
    pub peak: f32,
    pub rms: f32,
}

pub struct AudioRecorder {
    samples: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    channels: u16,
    device_name: String,
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
            device_name: String::new(),
            stream: None,
        })
    }

    pub fn start(&mut self, device_name: &str) -> Result<(), String> {
        if self.stream.is_some() {
            return Ok(());
        }

        let host = cpal::default_host();
        let device = input_device_by_name(&host, device_name)?;
        let config = device.default_input_config().map_err(|e| e.to_string())?;

        self.device_name = device
            .name()
            .unwrap_or_else(|_| "Unknown input device".to_string());
        self.sample_rate = config.sample_rate().0;
        self.channels = config.channels();
        self.samples.lock().unwrap().clear();

        let channels = self.channels as usize;
        let samples = Arc::clone(&self.samples);

        let stream = match config.sample_format() {
            SampleFormat::F32 => build_stream::<f32, _>(&device, &config, samples, channels, |s| s),
            SampleFormat::I16 => {
                build_stream::<i16, _>(&device, &config, samples, channels, |s| s as f32 / 32768.0)
            }
            SampleFormat::U16 => build_stream::<u16, _>(&device, &config, samples, channels, |s| {
                (s as f32 / 32768.0) - 1.0
            }),
            fmt => return Err(format!("Unsupported sample format: {fmt:?}")),
        }?;

        stream.play().map_err(|e| e.to_string())?;
        self.stream = Some(stream);
        Ok(())
    }

    pub fn stop_and_save(&mut self, path: &std::path::Path) -> Result<CaptureInfo, String> {
        self.stream = None; // dropping the stream stops capture

        let samples = self.samples.lock().unwrap();
        if samples.is_empty() {
            return Err("No audio captured".to_string());
        }

        let (peak, sum_squares) = samples.iter().fold((0.0f32, 0.0f32), |(peak, sum), &s| {
            let sample = clean_sample(s);
            (peak.max(sample.abs()), sum + sample * sample)
        });
        let rms = (sum_squares / samples.len() as f32).sqrt();
        let duration_secs = samples.len() as f32 / self.sample_rate.max(1) as f32;
        let info = CaptureInfo {
            device_name: self.device_name.clone(),
            sample_rate: self.sample_rate,
            input_channels: self.channels,
            samples: samples.len(),
            duration_secs,
            peak,
            rms,
        };

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: self.sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = hound::WavWriter::create(path, spec).map_err(|e| e.to_string())?;
        for &s in samples.iter() {
            let pcm = (clean_sample(s) * i16::MAX as f32).round() as i16;
            writer.write_sample(pcm).map_err(|e| e.to_string())?;
        }
        writer.finalize().map_err(|e| e.to_string())?;
        Ok(info)
    }

    pub fn is_recording(&self) -> bool {
        self.stream.is_some()
    }
}

pub fn list_input_devices() -> Result<Vec<AudioDeviceInfo>, String> {
    let host = cpal::default_host();
    let default_name = host.default_input_device().and_then(|d| d.name().ok());
    let mut devices = Vec::new();

    for device in host.input_devices().map_err(|e| e.to_string())? {
        let name = device
            .name()
            .unwrap_or_else(|_| "Unknown input device".to_string());
        if devices.iter().any(|d: &AudioDeviceInfo| d.id == name) {
            continue;
        }
        devices.push(AudioDeviceInfo {
            id: name.clone(),
            is_default: default_name.as_deref() == Some(name.as_str()),
            name,
        });
    }

    devices.sort_by(|a, b| {
        b.is_default
            .cmp(&a.is_default)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(devices)
}

fn input_device_by_name(host: &cpal::Host, device_name: &str) -> Result<cpal::Device, String> {
    if device_name.trim().is_empty() {
        return host
            .default_input_device()
            .ok_or_else(|| "No microphone found".to_string());
    }

    let requested = device_name.trim();
    for device in host.input_devices().map_err(|e| e.to_string())? {
        if device.name().ok().as_deref() == Some(requested) {
            return Ok(device);
        }
    }

    Err(format!("Selected microphone not found: {requested}"))
}

fn clean_sample(sample: f32) -> f32 {
    if sample.is_finite() {
        sample.clamp(-1.0, 1.0)
    } else {
        0.0
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
                        .map(|frame| {
                            frame
                                .iter()
                                .map(|&s| convert(s))
                                .max_by(|a, b| {
                                    a.abs()
                                        .partial_cmp(&b.abs())
                                        .unwrap_or(std::cmp::Ordering::Equal)
                                })
                                .unwrap_or(0.0)
                        })
                        .collect()
                };
                samples.lock().unwrap().extend(mono);
            },
            |e| eprintln!("Audio error: {e}"),
            None,
        )
        .map_err(|e| e.to_string())
}

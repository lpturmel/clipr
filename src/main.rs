use crate::cli::Cli;
use clap::Parser;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat, SupportedStreamConfig,
};
use device_query::{DeviceQuery, DeviceState, Keycode};
use ringbuf::{traits::*, HeapRb};
use std::{
    fs::File,
    io::BufWriter,
    path::Path,
    sync::{mpsc, Arc, Mutex},
    time::Duration,
};

mod cli;

type SaveRequest = Vec<f32>;

fn main() -> Result<(), anyhow::Error> {
    let app_dir =
        directories::UserDirs::new().ok_or(anyhow::Error::msg("Failed to get user directory"))?;
    let app_dir = app_dir
        .audio_dir()
        .ok_or(anyhow::Error::msg("Failed to get audio directory"))?;
    let app_dir = app_dir.join("clipr");

    let cli = Cli::parse();

    if !app_dir.exists() {
        std::fs::create_dir_all(&app_dir)?;
    }

    let host = cpal::default_host();
    let device = host
        .output_devices()?
        .find(|d| {
            d.name()
                .map(|y| y.as_str() == "BlackHole 2ch")
                .unwrap_or(false)
        })
        .ok_or(anyhow::Error::msg("'BlackHole 2ch' not available"))?;

    println!("Using device: {}", device.name()?);

    let config = device.default_input_config()?;

    let sample_rate = config.sample_rate().0 as usize;
    let sample_format = config.sample_format();
    let channels = config.channels() as usize;

    let total_samples = sample_rate * channels * cli.duration;
    let rb = Arc::new(Mutex::new(HeapRb::<f32>::new(total_samples)));

    let (tx, rx) = mpsc::channel::<SaveRequest>();

    let config_c = config.clone();
    let app_dir_c = app_dir.clone();
    let cli_c = cli.clone();

    std::thread::spawn(move || {
        for samples in rx {
            if let Err(e) = save_recording(&cli_c, &samples, &app_dir_c, &config_c) {
                eprintln!("Failed to save recording: {}", e);
            }
        }
    });

    let stream_rb = rb.clone();
    let stream = match sample_format {
        SampleFormat::F32 => device.build_input_stream(
            &config.clone().into(),
            move |data: &[f32], _| {
                let mut rb = stream_rb.lock().unwrap();
                rb.push_slice_overwrite(data);
            },
            |err| eprintln!("Stream error: {}", err),
            None,
        )?,
        _ => anyhow::bail!("unsupported sample format"),
    };

    stream.play()?;

    let device_state = DeviceState::new();

    loop {
        std::thread::sleep(Duration::from_millis(100));
        let keys = device_state.get_keys();

        if keys.contains(&Keycode::S)
            && keys.contains(&Keycode::LControl)
            && (keys.contains(&Keycode::LAlt) || keys.contains(&Keycode::LOption))
        {
            let samples = {
                // Make sure we drop the lock before sending the samples
                let mut rb = rb.lock().unwrap();
                rb.pop_iter().collect::<Vec<_>>()
            };

            let is_empty = samples.iter().all(|sample| sample.abs() < 1e-6);

            if is_empty {
                println!("Recording is empty, not saving.");
                continue;
            }

            if let Err(e) = tx.send(samples) {
                eprintln!("Failed to send recording: {}", e);
            }
        }
    }
}

fn save_recording(
    cli: &Cli,
    samples: &[f32],
    path: &Path,
    config: &SupportedStreamConfig,
) -> Result<(), anyhow::Error> {
    let spec = wav_spec_from_config(config);

    let filename = format!(
        "recorded_{}.wav",
        chrono::Local::now().format(cli.format.as_str())
    );
    let path = path.join(filename);
    let file = BufWriter::new(File::create(&path)?);
    let mut wav_writer = hound::WavWriter::new(file, spec)?;

    if let (Some(start), Some(end)) = (
        samples.iter().position(|&s| s.abs() >= 1e-6),
        samples.iter().rposition(|&s| s.abs() >= 1e-6),
    ) {
        for &sample in &samples[start..=end] {
            wav_writer.write_sample(sample)?;
        }
    } else {
        println!("Recording is empty, not saving.");
    }

    wav_writer.finalize()?;
    println!("Saved recording to '{}'.", path.display());
    Ok(())
}

fn sample_format(format: cpal::SampleFormat) -> hound::SampleFormat {
    if format.is_float() {
        hound::SampleFormat::Float
    } else {
        hound::SampleFormat::Int
    }
}

fn wav_spec_from_config(config: &cpal::SupportedStreamConfig) -> hound::WavSpec {
    hound::WavSpec {
        channels: config.channels() as _,
        sample_rate: config.sample_rate().0 as _,
        bits_per_sample: (config.sample_format().sample_size() * 8) as _,
        sample_format: sample_format(config.sample_format()),
    }
}

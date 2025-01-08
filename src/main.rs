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
    sync::{Arc, Mutex},
    time::Duration,
};

use self::cli::Cli;

mod cli;

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

    println!(
        "Using device: {}",
        device.name().expect("Invalid device name")
    );

    let config = device.default_input_config()?;

    let sample_rate = config.sample_rate().0 as usize;
    let sample_format = config.sample_format();
    let channels = config.channels() as usize;

    let total_samples = sample_rate * channels * cli.duration;
    let rb = Arc::new(Mutex::new(HeapRb::<f32>::new(total_samples)));

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
        let keys = device_state.get_keys();

        if keys.contains(&Keycode::S)
            && keys.contains(&Keycode::LControl)
            && (keys.contains(&Keycode::LAlt) || keys.contains(&Keycode::LOption))
        {
            let mut rb = rb.lock().unwrap();
            let samples = rb.pop_iter().collect::<Vec<_>>();

            let _ = save_recording(&cli, samples, &app_dir, &config);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn save_recording(
    cli: &Cli,
    samples: Vec<f32>,
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

    for sample in samples {
        wav_writer.write_sample(sample)?;
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

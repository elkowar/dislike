use std::sync::mpsc::Sender;

use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

pub fn init_listening(send: Sender<Vec<i16>>) -> Result<()> {
    let host = cpal::default_host();

    let device = host
        .input_devices()
        .context("Failed to find input devices")?
        .find(|d| d.name().unwrap() == "pipewire" || d.name().unwrap() == "pulse")
        .context("Failed to find supported input device")?;
    let input_config = device
        .supported_input_configs()
        .context("Failed to get supported input-config")?
        .find(|c| {
            c.channels() == 1
                && c.min_sample_rate() <= cpal::SampleRate(16_000)
                && c.max_sample_rate() >= cpal::SampleRate(16_000)
                && c.sample_format() == cpal::SampleFormat::I16
        })
        .context("Failed to get single-channel config")?
        .with_sample_rate(cpal::SampleRate(16_000));

    let stream = device.build_input_stream(
        &input_config.into(),
        move |data: &[i16], _: &_| send.send(data.to_vec()).unwrap(),
        move |err| eprintln!("an error occurred on stream: {}", err),
    )?;

    stream.play()?;
    Ok(())
}

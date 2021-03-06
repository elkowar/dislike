use std::sync::mpsc::{Receiver, Sender};

use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

pub struct MicInput {
    recv_samples: Receiver<Vec<i16>>,
    send_stop: Sender<()>,
}
impl MicInput {
    pub fn init() -> Result<MicInput> {
        let (send_samples, recv_samples) = std::sync::mpsc::channel();
        let (send_stop, recv_stop) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let result = spawn_listen_thread(send_samples, recv_stop);
            if let Err(err) = result {
                eprintln!("Error listening to microphone input: {}", err)
            }
        });

        Ok(MicInput {
            recv_samples,
            send_stop,
        })
    }

    pub fn stop(&self) {
        if let Err(err) = self.send_stop.send(()) {
            eprintln!("Error sending stop signal: {}", err)
        }
    }

    pub fn wait_for_samples(&self) -> Result<Vec<i16>> {
        self.recv_samples
            .recv()
            .context("Error receiving samples from microphone input")
    }
}

impl Drop for MicInput {
    fn drop(&mut self) {
        self.stop();
    }
}

fn spawn_listen_thread(send_samples: Sender<Vec<i16>>, recv_stop: Receiver<()>) -> Result<()> {
    let host = cpal::default_host();

    let device = host
        .default_input_device()
        .context("Failed to find default input device")?;
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
        move |data: &[i16], _: &_| send_samples.send(data.to_vec()).unwrap(),
        move |err| eprintln!("an error occurred on stream: {}", err),
    )?;

    stream.play().unwrap();

    // Wait for stop signal
    if let Err(err) = recv_stop.recv() {
        eprintln!("Error receiving stop signal: {}", err);
    }
    stream.pause().unwrap();
    drop(stream);

    Ok(())
}


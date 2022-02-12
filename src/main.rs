use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

fn main() -> Result<()> {
    let host = cpal::default_host();
    let device = host
        .input_devices()?
        .find(|x| x.name().map(|name| name == "pipewire").unwrap_or(false))
        .context("Failed to find pipewire device")?;

    let config = device
        .supported_input_configs()?
        .find(|x| x.channels() == 1)
        .context("Failed to get single-channel config")?
        .with_sample_rate(cpal::SampleRate(16_000));

    let err_fn = move |err| {
        eprintln!("an error occurred on stream: {}", err);
    };

    let (send, recv) = std::sync::mpsc::channel::<Vec<i16>>();

    std::thread::spawn(move || {
        let mut sample_handler = SampleHandler::new().unwrap();
        let mut sample_cnt = 0;
        while let Ok(data) = recv.recv() {
            sample_handler.on_samples(&data).unwrap();
            sample_cnt += 1;
            if sample_cnt > 1000 {
                sample_handler.flush().unwrap();
                sample_cnt = 0;
            }
        }
    });

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config.into(),
            move |data: &[f32], _: &_| print_samples(data),
            err_fn,
        )?,
        cpal::SampleFormat::I16 => device.build_input_stream(
            &config.into(),
            move |data: &[i16], _: &_| send.send(data.to_vec()).unwrap(),
            err_fn,
        )?,
        cpal::SampleFormat::U16 => device.build_input_stream(
            &config.into(),
            move |data: &[u16], _: &_| print_samples(data),
            err_fn,
        )?,
    };
    stream.play()?;
    //std::thread::sleep_ms(10000);
    loop {}
    drop(stream);

    Ok(())
}
fn print_samples<T>(input: &[T])
where
    T: cpal::Sample + std::fmt::Debug,
{
    println!("{:?}", input);
}

struct SampleHandler {
    model: deepspeech::Model,
    stream: deepspeech::Stream,
}
impl SampleHandler {
    pub fn new() -> Result<Self> {
        let mut model = deepspeech::Model::load_from_files(&std::path::PathBuf::from(
            "./deepspeech_model/deepspeech-0.9.0-models.pbmm",
        ))?;

        let stream = model.create_stream()?;

        Ok(Self { model, stream })
    }

    fn on_samples(&mut self, data: &[i16]) -> Result<()> {
        self.stream.feed_audio(data);
        if let Ok(x) = self.stream.intermediate_decode() {
            if let Some(word) = x.split(' ').last() {
                println!("{}", word);
            }

            if x.contains("like") || x.contains("i mean") || x.contains("i guess") {
                println!("fuck you");
                std::process::Command::new("espeak")
                    .args(["fuck you"])
                    .spawn()
                    .unwrap();
                self.flush()?;
            }
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        let old_stream = std::mem::replace(&mut self.stream, self.model.create_stream()?);
        old_stream.finish()?;
        Ok(())
    }
}

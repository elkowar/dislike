mod mic_input;
mod opts;
use std::{io::Write, process::Command};

use anyhow::Result;

use opts::*;

fn main() -> Result<()> {
    let opts = Opts::from_cli();

    let mic_input = mic_input::MicInput::init()?;

    let on_trigger = {
        let command_opt = opts.command.clone();
        move |word: &str| {
            if let Some(cmd) = &command_opt {
                let process = Command::new("bash")
                    .arg("-c")
                    .arg(cmd)
                    .stdin(std::process::Stdio::piped())
                    .spawn()?;
                process.stdin.unwrap().write_all(word.as_bytes())?;
            }
            Ok(())
        }
    };

    let mut sample_handler = SampleHandler::new(&opts, Box::new(on_trigger))?;
    let mut sample_cnt = 0;
    while let Ok(data) = mic_input.wait_for_samples() {
        sample_handler.on_samples(&data).unwrap();
        sample_cnt += 1;
        if sample_cnt > 1000 {
            sample_handler.flush().unwrap();
            sample_cnt = 0;
        }
    }

    mic_input.stop();

    Ok(())
}

struct SampleHandler {
    model: deepspeech::Model,
    stream: deepspeech::Stream,
    trigger_words: Vec<String>,
    last_word: String,
    on_trigger: Box<dyn Fn(&str) -> Result<()>>,
}
impl SampleHandler {
    pub fn new(opts: &Opts, on_trigger: Box<dyn Fn(&str) -> Result<()>>) -> Result<Self> {
        let mut model = deepspeech::Model::load_from_files(&opts.model_path)?;

        let stream = model.create_stream()?;

        Ok(Self {
            model,
            stream,
            trigger_words: opts.words.clone(),
            last_word: String::new(),
            on_trigger,
        })
    }

    fn on_samples(&mut self, data: &[i16]) -> Result<()> {
        self.stream.feed_audio(data);
        if let Ok(decode_result) = self.stream.intermediate_decode() {
            if let Some(word) = decode_result.split(' ').last() {
                if word != self.last_word {
                    self.last_word = word.to_string();
                    println!("{}", word);
                }
            }

            let triggered_word = self
                .trigger_words
                .iter()
                .find(|word| decode_result.contains(*word));
            if let Some(word) = triggered_word {
                println!("triggered because of: {}", word);
                if let Err(err) = (*self.on_trigger)(word) {
                    eprintln!("Error running on-trigger command: {}", err);
                }
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


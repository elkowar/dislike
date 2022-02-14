mod mic_input;
mod opts;
use anyhow::Result;

use opts::*;

fn main() -> Result<()> {
    let opts = Opts::from_cli();

    let (send, recv) = std::sync::mpsc::channel::<Vec<i16>>();
    let listener = std::thread::spawn(move || match mic_input::init_listening(send) {
        Ok(_) => println!("Stopped listening"),
        Err(e) => eprintln!("{}", e),
    });

    let mut sample_handler = SampleHandler::new(&opts).unwrap();
    let mut sample_cnt = 0;
    while let Ok(data) = recv.recv() {
        sample_handler.on_samples(&data).unwrap();
        sample_cnt += 1;
        if sample_cnt > 1000 {
            sample_handler.flush().unwrap();
            sample_cnt = 0;
        }
    }
    listener.join().unwrap();

    Ok(())
}

struct SampleHandler {
    model: deepspeech::Model,
    stream: deepspeech::Stream,
    words: Vec<String>,
}
impl SampleHandler {
    pub fn new(opts: &Opts) -> Result<Self> {
        let mut model = deepspeech::Model::load_from_files(&opts.model_path)?;

        let stream = model.create_stream()?;

        Ok(Self {
            model,
            stream,
            words: opts.words.clone(),
        })
    }

    fn on_samples(&mut self, data: &[i16]) -> Result<()> {
        self.stream.feed_audio(data);
        if let Ok(x) = self.stream.intermediate_decode() {
            if let Some(word) = x.split(' ').last() {
                println!("{}", word);
            }

            if self.words.iter().any(|word| x.contains(word)) {
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


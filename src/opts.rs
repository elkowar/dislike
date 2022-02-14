use clap::{arg, App};
#[derive(Debug)]
pub struct Opts {
    pub words: Vec<String>,
    pub model_path: std::path::PathBuf,
}
impl Opts {
    pub fn from_cli() -> Self {
        let app = App::new("dislike");
        let matches = app
            .arg(arg!(-m --model <FILE> "Path to the deepspeech-model.pbmm file").required(true))
            .arg(
                arg!(-w --words <WORDs> "Specify trigger words. If none are specified, defaults to \"like\"")
                    .multiple_values(true)
                    .required(false)
                    .default_value("like")
            )
            .get_matches();

        Self {
            words: matches
                .values_of("words")
                .unwrap()
                .into_iter()
                .map(|x| x.to_string())
                .collect(),
            model_path: matches.value_of("model").unwrap().into(),
        }
    }
}


use clap::Parser;

#[derive(Parser, Clone)]
#[clap(author = "Louis-Philippe Turmel", version, about, long_about = None)]
pub struct Cli {
    #[clap(short, long, default_value = "30")]
    pub duration: usize,

    #[clap(short, long, default_value = "%Y%m%d_%H%M%S")]
    pub format: String,
}

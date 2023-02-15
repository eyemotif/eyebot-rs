use clap::Parser;

#[derive(Parser)]
#[command(name = "eyebot-rs")]
#[command(author, version)]
#[command(about = "A Rust-powered Twitch bot.")]
pub struct Cli {
    #[arg(long)]
    pub oauth: Option<String>,
    #[arg(long)]
    pub clientid: String,
    #[arg(long)]
    pub clientsecret: String,
    #[arg(long)]
    pub store: Option<String>,
    #[arg(long)]
    pub reauth: bool,
}

use clap::Parser;

#[derive(Parser)]
#[command(name = "eyebot-rs")]
#[command(author, version)]
#[command(about = "A Rust-powered Twitch bot.")]
pub struct Cli {
    #[arg(long)]
    pub oauth: Option<String>,
    #[arg(short = 'i', long)]
    pub clientid: String,
    #[arg(short = 's', long)]
    pub clientsecret: String,
    #[arg(short = 'c', long = "chat-access")]
    pub chat_access: Option<String>,
    #[arg(long)]
    pub store: Option<String>,
    #[arg(long)]
    pub reauth: bool,
    #[arg(short = 'o', long = "options-file")]
    pub options_file: Option<String>,
}

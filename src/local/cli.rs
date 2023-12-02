use clap::{ArgAction, Parser};

#[derive(Debug, Parser)]
#[command(name = "discfs")]
#[command(author = "sqooid")]
#[command(version = "0.1")]
#[command(about = "Mounts a virtual filesystem with files stored as Discord file uploads")]
pub struct Cli {
    /// Use dotenv-vault (https://www.dotenv.org/docs/)
    #[arg(long)]
    pub dotenv: bool,

    #[arg(short, action = ArgAction::Count)]
    pub verbosity: u8,

    /// Path to mount virtual filesystem at
    pub mountpoint: String,

    /// Path to create SQLite database file
    #[arg(long, default_value = "./fs.db", env = "DB_PATH")]
    pub db_path: String,
}

pub mod client;
pub mod local;
pub mod util;

use std::error::Error;

use clap::Parser;
use fuser::MountOption;
use local::{db::FsDatabase, fuse::DiscFs};
use log::{debug, info, LevelFilter};

use crate::local::cli::Cli;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    env_logger::builder()
        .filter_level(match cli.verbosity {
            0 => LevelFilter::Warn,
            1 => LevelFilter::Info,
            2 => LevelFilter::Debug,
            _ => LevelFilter::Trace,
        })
        .init();
    info!("starting up");
    debug!("cli config: {:?}", &cli);

    if cli.dotenv {
        let _ = dotenv_vault::dotenv();
    }

    let fs_database = FsDatabase::new(&cli.db_path).await?;
    let fs = DiscFs::new(fs_database)?;
    let mount_options = [
        MountOption::NoDev,
        MountOption::NoSuid,
        MountOption::NoExec,
        MountOption::AllowRoot,
        MountOption::AutoUnmount,
    ];
    let _ = fuser::mount2(fs, cli.mountpoint, &mount_options);

    Ok(())
}

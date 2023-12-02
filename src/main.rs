pub mod client;
pub mod local;
pub mod util;

use std::error::Error;

use clap::Parser;
use fuser::MountOption;
use local::{db::FsDatabase, fuse::DiscFs};
use log::{debug, info};

use crate::local::cli::Cli;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    info!("starting up");

    let cli = Cli::parse();
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

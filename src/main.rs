pub mod client;
pub mod encryption;
pub mod error;
pub mod local;
pub mod util;

use std::error::Error;

use clap::Parser;
use fuser::MountOption;
use local::{db::FsDatabase, fuse::DiscFs};
use log::{debug, info, LevelFilter};

use crate::local::{cli::Cli, fuse::CloudType};

fn main() -> Result<(), Box<dyn Error>> {
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

    let rt = tokio::runtime::Runtime::new()?;

    let fs_database = rt.block_on(async { FsDatabase::new(&cli.db_path).await })?;
    let fs = DiscFs::new(rt.handle().to_owned(), fs_database, CloudType::Discord)?;
    let mount_options = [
        MountOption::NoDev,
        MountOption::NoSuid,
        MountOption::NoExec,
        MountOption::AllowRoot,
        MountOption::AutoUnmount,
        MountOption::DefaultPermissions,
        MountOption::Async,
    ];

    rt.block_on(async {
        let _ = fuser::mount2(fs, cli.mountpoint, &mount_options);
    });

    Ok(())
}

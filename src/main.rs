pub mod client;
pub mod local;

use std::error::Error;

use fuser::MountOption;
use local::{db::FsDatabase, fuse::DiscFs};

// #[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {
//     let resp = reqwest::get("https://httpbin.org/ip")
//         .await?
//         .json::<HashMap<String, String>>()
//         .await?;
//     println!("{:#?}", resp);
//     Ok(())
// }

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let fs_database = FsDatabase::new().await?;
    let fs = DiscFs::new(fs_database);
    let mount_options = [
        MountOption::NoDev,
        MountOption::NoSuid,
        MountOption::NoExec,
        MountOption::AllowRoot,
        MountOption::AutoUnmount,
    ];
    let _ = fuser::mount2(fs, "/home/lucas/fusetest", &mount_options);

    Ok(())
}

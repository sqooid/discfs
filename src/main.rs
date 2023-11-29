pub mod client;
pub mod local;

use fuser::MountOption;
use local::fuse::DiscFs;

// #[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {
//     let resp = reqwest::get("https://httpbin.org/ip")
//         .await?
//         .json::<HashMap<String, String>>()
//         .await?;
//     println!("{:#?}", resp);
//     Ok(())
// }

fn main() {
    let fs = DiscFs {};
    let mount_options = [
        MountOption::NoDev,
        MountOption::NoSuid,
        MountOption::NoExec,
        MountOption::AllowRoot,
        MountOption::AutoUnmount,
    ];
    let _ = fuser::mount2(fs, "/home/lucas/code/discfs/testmnt", &mount_options);
}

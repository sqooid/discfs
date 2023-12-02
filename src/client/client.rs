use std::io::{Read, Write};

use crate::local::db::FsNode;


pub trait CloudFile: Read + Write {
    fn node(&self) -> &FsNode;
}

pub trait CloudClient {
    fn list_files(path: &str);
}

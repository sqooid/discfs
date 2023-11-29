use std::io::{Read, Write};
pub trait CloudFile: Read + Write {}

pub trait CloudClient {
    fn list_files(path: &str);
}

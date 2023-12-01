use fuser::{FileAttr, FileType};

use crate::local::{db::FsNode, error::FsError};

use super::time::float_to_time;

pub fn attrs_from_node(node: &FsNode) -> Result<FileAttr, FsError> {
    let ctime = float_to_time(node.ctime.unwrap_or(0.0))?;
    Ok(FileAttr {
        ino: node.id as u64,
        size: node.size.unwrap_or(0) as u64,
        blocks: 0,
        atime: float_to_time(node.atime.unwrap_or(0.0))?,
        mtime: ctime,
        ctime,
        crtime: ctime,
        kind: if node.directory {
            FileType::Directory
        } else {
            FileType::RegularFile
        },
        perm: 0b111111111,
        nlink: 1,
        uid: 0,
        gid: 0,
        rdev: 0,
        blksize: 0,
        flags: 0,
    })
}

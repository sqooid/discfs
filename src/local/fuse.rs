use fuser::Filesystem;
use libc::ENOENT;

use super::db::FsDatabase;

pub struct DiscFs {
    db: FsDatabase,
}

impl DiscFs {
    pub fn new(db: FsDatabase) -> Self {
        Self { db }
    }
}

impl Filesystem for DiscFs {
    fn lookup(
        &mut self,
        _req: &fuser::Request<'_>,
        parent: u64,
        name: &std::ffi::OsStr,
        reply: fuser::ReplyEntry,
    ) {
    }
}

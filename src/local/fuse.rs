use std::time::{Duration, SystemTime};

use fuser::{FileAttr, FileType, Filesystem};
use libc::ENOENT;
use log::debug;
use tokio::runtime::Runtime;

use crate::{local::error::DbError, util::time::float_to_time};

use super::{db::FsDatabase, error::FsError, virtual_fs::VirtualFs};

pub struct DiscFs {
    db: FsDatabase,
    rt: Runtime,
}

impl DiscFs {
    pub fn new(db: FsDatabase) -> Result<Self, FsError> {
        Ok(Self {
            db,
            rt: tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|e| FsError::RuntimeError(e.to_string()))?,
        })
    }
}

impl VirtualFs for DiscFs {}

impl Filesystem for DiscFs {
    fn lookup(
        &mut self,
        _req: &fuser::Request<'_>,
        parent: u64,
        name: &std::ffi::OsStr,
        reply: fuser::ReplyEntry,
    ) {
        debug!("lookup(parent: {:#x?}, name {:?})", parent, name);
        let _result: Result<(), FsError> = self.rt.block_on(async {
            let node = self
                .db
                .get_node(parent, name)
                .await
                .map_err(|e| FsError::DatabaseError(e))?;
            match node {
                Some(n) => reply.entry(&Duration::from_millis(64), &self.attrs_from_node(&n)?, 0),
                None => reply.error(ENOENT),
            }
            Ok(())
        });
    }
}

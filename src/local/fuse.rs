use std::{sync::Arc, time::Duration};

use fuser::Filesystem;
use libc::ENOENT;
use log::debug;
use tokio::{spawn, task::JoinHandle};

use crate::util::fs::attrs_from_node;

use super::{db::FsDatabase, error::FsError};

pub struct DiscFs {
    db: Arc<FsDatabase>,
}

impl DiscFs {
    pub fn new(db: FsDatabase) -> Result<Self, FsError> {
        Ok(Self { db: Arc::new(db) })
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
        debug!("lookup(parent: {:#x?}, name {:?})", parent, name);

        let db = Arc::clone(&self.db);
        let name_cp = name.to_owned();
        let _: JoinHandle<Result<(), FsError>> = spawn(async move {
            let node = db
                .get_node(parent, &name_cp)
                .await
                .map_err(|e| FsError::DatabaseError(e))?;
            match node {
                Some(n) => reply.entry(&Duration::from_millis(64), &attrs_from_node(&n)?, 0),
                None => reply.error(ENOENT),
            }
            Ok(())
        });
    }
}

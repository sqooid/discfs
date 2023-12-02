use std::{sync::Arc, time::Duration};

use fuser::Filesystem;
use libc::ENOENT;
use log::debug;
use tokio::{spawn, task::JoinHandle};

use crate::{
    client::{client::CloudClient, discord::DiscordClient},
    util::fs::attrs_from_node,
};

use super::{db::FsDatabase, error::FsError};

pub struct DiscFs {
    db: Arc<FsDatabase>,
    ctype: CloudType,
    client: Arc<dyn CloudClient>,
}

impl DiscFs {
    pub fn new(db: FsDatabase, ctype: CloudType) -> Result<Self, FsError> {
        Ok(Self {
            db: Arc::new(db),
            client: Arc::new(match ctype {
                CloudType::Discord => DiscordClient::new()?,
            }),
            ctype,
        })
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

    fn mkdir(
        &mut self,
        _req: &fuser::Request<'_>,
        parent: u64,
        name: &std::ffi::OsStr,
        mode: u32,
        umask: u32,
        reply: fuser::ReplyEntry,
    ) {
        debug!(
            "[Not Implemented] mkdir(parent: {:#x?}, name: {:?}, mode: {}, umask: {:#x?})",
            parent, name, mode, umask
        );
        reply.error(ENOSYS);
    }
}

pub enum CloudType {
    Discord,
}

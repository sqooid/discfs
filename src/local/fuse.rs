use std::{sync::Arc, time::Duration};

use fuser::Filesystem;
use libc::{c_int, EEXIST, ENOENT};
use log::debug;
use tokio::{runtime::Handle, spawn, task::JoinHandle};

use crate::{
    client::{client::CloudClient, discord::client::DiscordClient},
    local::{db::FsNode, error::DbError},
    util::fs::attrs_from_node,
};

use super::{db::FsDatabase, error::FsError};

const EUNKNOWN: c_int = 99;

pub struct DiscFs {
    db: Arc<FsDatabase>,
    ctype: CloudType,
    client: Arc<dyn CloudClient>,
    rt: Handle,
}

impl DiscFs {
    pub fn new(rt: Handle, db: FsDatabase, ctype: CloudType) -> Result<Self, FsError> {
        Ok(Self {
            rt,
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
        let node = self
            .rt
            .block_on(async { db.get_node(parent, &name_cp).await });
        match node {
            Ok(n) => match n {
                Some(n) => {
                    if let Ok(attrs) = &attrs_from_node(&n) {
                        reply.entry(&Duration::from_millis(64), attrs, 0)
                    } else {
                        reply.error(EUNKNOWN)
                    }
                }
                None => reply.error(ENOENT),
            },
            Err(_) => reply.error(EUNKNOWN),
        };
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
            "mkdir(parent: {:#x?}, name: {:?}, mode: {}, umask: {:#x?})",
            parent, name, mode, umask
        );
        let db = Arc::clone(&self.db);
        let name = name.to_owned();
        let node = self
            .rt
            .block_on(async { db.create_node(parent, &name, true).await });
        match node {
            Ok(n) => {
                if let Ok(attrs) = &attrs_from_node(&n) {
                    reply.entry(&Duration::from_millis(64), attrs, 0)
                } else {
                    reply.error(EUNKNOWN)
                }
            }
            Err(e) => match e {
                crate::local::error::DbError::Exists(_, _) => reply.error(EEXIST),
                _ => reply.error(EUNKNOWN),
            },
        }
    }

    fn mknod(
        &mut self,
        _req: &fuser::Request<'_>,
        parent: u64,
        name: &std::ffi::OsStr,
        mode: u32,
        umask: u32,
        rdev: u32,
        reply: fuser::ReplyEntry,
    ) {
    }
}

pub enum CloudType {
    Discord,
}

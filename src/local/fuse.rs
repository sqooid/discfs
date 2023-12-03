use std::{
    collections::{HashMap, HashSet},
    io::Write,
    result,
    sync::Arc,
    time::Duration,
};

use fuser::{FileType, Filesystem};
use libc::{c_int, EEXIST, ENOENT};
use log::{debug, error};
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
    write_handles: HashMap<u64, Box<dyn std::io::Write>>,
    read_handles: HashMap<u64, Box<dyn std::io::Read>>,
    started_dirs: HashSet<u64>,
}

impl DiscFs {
    pub fn new(rt: Handle, db: FsDatabase, ctype: CloudType) -> Result<Self, FsError> {
        let db = Arc::new(db);
        Ok(Self {
            db: db.clone(),
            client: Arc::new(match ctype {
                CloudType::Discord => DiscordClient::new(rt.clone(), db)?,
            }),
            rt,
            ctype,
            write_handles: HashMap::new(),
            read_handles: HashMap::new(),
            started_dirs: HashSet::new(),
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
        debug!(
            "mknod(parent: {:#x?}, name: {:?}, mode: {}, \
            umask: {:#x?}, rdev: {})",
            parent, name, mode, umask, rdev
        );
        let node = self
            .rt
            .block_on(async { self.db.create_node(parent, name, false).await });
        match node {
            Ok(n) => match attrs_from_node(&n) {
                Ok(attrs) => reply.entry(&Duration::from_millis(64), &attrs, 0),
                Err(e) => {
                    error!("error in mknod: {:?}", e);
                    reply.error(EUNKNOWN)
                }
            },
            Err(e) => match e {
                DbError::Exists(_, _) => reply.error(EEXIST),
                _ => reply.error(EUNKNOWN),
            },
        }
    }

    fn open(&mut self, _req: &fuser::Request<'_>, ino: u64, flags: i32, reply: fuser::ReplyOpen) {
        let node = self
            .rt
            .block_on(async { self.db.get_node_by_id(ino).await });
        match node {
            Ok(n) => match n {
                Some(n) => {
                    if flags & 1 != 0 {
                        let file = self.client.open_file_write(n.clone());
                        self.write_handles.insert(n.id as u64, file);
                        reply.opened(0, 0b110110110)
                    } else {
                        if let Ok(file) = self.client.open_file_read(n.clone()) {
                            self.read_handles.insert(n.id as u64, file);
                            reply.opened(0, 0b100100100)
                        } else {
                            reply.error(EUNKNOWN);
                        }
                    }
                }
                None => reply.error(ENOENT),
            },
            Err(_) => reply.error(EUNKNOWN),
        }
    }

    fn write(
        &mut self,
        _req: &fuser::Request<'_>,
        ino: u64,
        fh: u64,
        offset: i64,
        data: &[u8],
        write_flags: u32,
        flags: i32,
        lock_owner: Option<u64>,
        reply: fuser::ReplyWrite,
    ) {
        debug!(
            "write(ino: {:#x?}, fh: {}, offset: {}, data.len(): {}, \
            write_flags: {:#x?}, flags: {:#x?}, lock_owner: {:?})",
            ino,
            fh,
            offset,
            data.len(),
            write_flags,
            flags,
            lock_owner
        );
        let file = self.write_handles.get_mut(&ino);
        if let Some(handle) = file {
            // Without encryption for now
            if let Ok(written) = handle.write(data) {
                reply.written(written as u32)
            } else {
                reply.error(EUNKNOWN)
            };
        } else {
            reply.error(EUNKNOWN);
        }
    }

    fn getattr(&mut self, _req: &fuser::Request<'_>, ino: u64, reply: fuser::ReplyAttr) {
        let result = self
            .rt
            .block_on(async { self.db.get_node_by_id(ino).await });
        let node = result.unwrap_or(None);
        if let Some(node) = node {
            if let Ok(attrs) = attrs_from_node(&node) {
                reply.attr(&Duration::from_millis(64), &attrs);
            } else {
                reply.error(EUNKNOWN)
            }
        } else {
            reply.error(ENOENT)
        };
    }

    fn release(
        &mut self,
        _req: &fuser::Request<'_>,
        ino: u64,
        _fh: u64,
        _flags: i32,
        _lock_owner: Option<u64>,
        _flush: bool,
        reply: fuser::ReplyEmpty,
    ) {
        if let Some(file) = self.write_handles.get_mut(&ino) {
            match file.flush() {
                Ok(_) => reply.ok(),
                Err(_) => reply.error(EUNKNOWN),
            }
        } else {
            reply.error(ENOENT)
        }
    }

    fn readdir(
        &mut self,
        _req: &fuser::Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: fuser::ReplyDirectory,
    ) {
        if self.started_dirs.contains(&ino) {
            self.started_dirs.remove(&ino);
            reply.ok();
            return;
        }
        debug!("readdir ino: {:?} offset: {:?}", ino, offset);
        let result = self
            .rt
            .block_on(async { self.db.get_nodes_by_parent(ino as i64).await });
        if let Ok(nodes) = result {
            debug!("files {:?}", &nodes);
            let mut full = false;
            let mut i = offset as usize;
            while !full && i < nodes.len() {
                let node = &nodes[i];
                full = reply.add(
                    node.id as u64,
                    0,
                    if node.directory {
                        FileType::Directory
                    } else {
                        FileType::RegularFile
                    },
                    node.name.clone().unwrap_or_else(|| "".to_string()),
                );
                i += 1;
            }
            self.started_dirs.insert(ino);
            reply.ok();
        } else {
            reply.error(EUNKNOWN);
        }
    }

    fn read(
        &mut self,
        _req: &fuser::Request<'_>,
        ino: u64,
        fh: u64,
        offset: i64,
        size: u32,
        flags: i32,
        lock_owner: Option<u64>,
        reply: fuser::ReplyData,
    ) {
        debug!(
            "read(ino: {:#x?}, fh: {}, offset: {}, size: {}, \
            flags: {:#x?}, lock_owner: {:?})",
            ino, fh, offset, size, flags, lock_owner
        );
    }
}

pub enum CloudType {
    Discord,
}

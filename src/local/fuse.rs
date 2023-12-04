use std::{
    collections::{HashMap, HashSet},
    io::Write,
    result,
    sync::Arc,
    time::Duration,
};

use fuser::{FileType, Filesystem};
use libc::{c_int, EEXIST, EIO, ENOENT, ENONET, ENOSYS};
use log::{debug, error, info, trace};
use tokio::{runtime::Handle, spawn, task::JoinHandle};

use crate::{
    client::{
        client::{CloudClient, CloudRead, CloudWrite},
        discord::client::DiscordClient,
    },
    local::{db::FsNode, error::DbError},
    util::fs::attrs_from_node,
};

use super::{db::FsDatabase, error::FsError};

const EUNKNOWN: c_int = 99;

// Unused open flags
// const FOPEN_DIRECT_IO: u32 = 1 << 0;
// const FOPEN_KEEP_CACHE: u32 = 1 << 1;
// const FOPEN_NONSEEKABLE: u32 = 1 << 2;
// const FOPEN_CACHE_DIR: u32 = 1 << 3;
// const FOPEN_STREAM: u32 = 1 << 4;
// const FOPEN_NOFLUSH: u32 = 1 << 5;
// const FOPEN_PARALLEL_DIRECT_WRITES: u32 = 1 << 6;

pub struct DiscFs {
    db: Arc<FsDatabase>,
    client: Arc<dyn CloudClient>,
    rt: Handle,
    write_handles: HashMap<u64, Box<dyn CloudWrite>>,
    read_handles: HashMap<u64, Box<dyn CloudRead>>,
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
        info!("create directory: {:?}", name);
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
                        info!(
                            "create file: {}",
                            n.name.as_ref().unwrap_or(&"".to_string())
                        );
                        let file = self.client.open_file_write(n.clone());
                        self.write_handles.insert(n.id as u64, file);
                        reply.opened(0, 0);
                    } else {
                        if let Ok(file) = self.client.open_file_read(n.clone()) {
                            info!("read file: {}", n.name.as_ref().unwrap_or(&"".to_string()));
                            self.read_handles.insert(n.id as u64, file);
                            reply.opened(0, 0);
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
        flags: i32,
        _lock_owner: Option<u64>,
        _flush: bool,
        reply: fuser::ReplyEmpty,
    ) {
        if flags & 1 != 0 {
            if let Some(file) = self.write_handles.get_mut(&ino) {
                match file.flush() {
                    Ok(_) => reply.ok(),
                    Err(_) => reply.error(EUNKNOWN),
                }
                self.write_handles.remove(&ino);
            } else {
                reply.error(ENOENT)
            }
        } else {
            if let Some(handle) = self.read_handles.remove(&ino) {
                handle.finish();
                reply.ok();
            } else {
                reply.error(ENOENT);
            }
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
            debug!("readdir done");
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
        _fh: u64,
        _offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: fuser::ReplyData,
    ) {
        if let Some(handle) = self.read_handles.get_mut(&ino) {
            let mut buffer = vec![0; size as usize].into_boxed_slice();
            let result = handle.read(&mut buffer);
            if let Ok(written) = result {
                debug!("written: {:?}", written);
                reply.data(&buffer[..written]);
            } else {
                reply.error(EUNKNOWN);
            }
        } else {
            reply.error(ENOENT);
        }
    }

    fn rmdir(
        &mut self,
        _req: &fuser::Request<'_>,
        parent: u64,
        name: &std::ffi::OsStr,
        reply: fuser::ReplyEmpty,
    ) {
        let result = self.rt.block_on(async {
            self.db
                .delete_dir(parent as i64, &name.to_string_lossy())
                .await
        });
        if let Ok(deleted) = result {
            if deleted == 0 {
                reply.error(ENOENT);
            } else {
                reply.ok();
            }
        } else {
            reply.error(EUNKNOWN);
        }
    }

    fn rename(
        &mut self,
        _req: &fuser::Request<'_>,
        parent: u64,
        name: &std::ffi::OsStr,
        newparent: u64,
        newname: &std::ffi::OsStr,
        flags: u32,
        reply: fuser::ReplyEmpty,
    ) {
        let result = self.rt.block_on(async {
            self.db
                .move_node(
                    parent as i64,
                    &name.to_string_lossy(),
                    newparent as i64,
                    &newname.to_string_lossy(),
                )
                .await
        });
        match result {
            Ok(_) => reply.ok(),
            Err(DbError::DoesNotExist(_)) => reply.error(ENOENT),
            Err(_) => reply.error(EUNKNOWN),
        }
    }
}

pub enum CloudType {
    Discord,
}

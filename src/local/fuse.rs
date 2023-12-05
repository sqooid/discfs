use std::{collections::HashMap, io::Write, sync::Arc, time::Duration};

use fuser::{FileType, Filesystem};
use libc::{c_int, EEXIST, ENOENT};
use log::{debug, error, info};
use tokio::{runtime::Handle, sync::Mutex};

use crate::{
    client::{
        client::{CloudClient, CloudRead, CloudWrite},
        discord::client::{DiscordClient},
    },
    local::error::DbError,
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
    rt: Handle,
    inner: Arc<DiscFsInner>,
}

pub struct DiscFsInner {
    pub write_handles: Arc<Mutex<HashMap<u64, Box<dyn CloudWrite>>>>,
    pub read_handles: Arc<Mutex<HashMap<u64, Box<dyn CloudRead>>>>,
    pub db: Arc<FsDatabase>,
    pub client: Box<dyn CloudClient>,
}

impl DiscFs {
    pub fn new(rt: Handle, db: FsDatabase, ctype: CloudType) -> Result<Self, FsError> {
        let db = Arc::new(db);
        let inner = DiscFsInner {
            db: db.clone(),
            client: Box::new(match ctype {
                CloudType::Discord => DiscordClient::new(rt.clone(), db)?,
            }),
            write_handles: Arc::new(Mutex::new(HashMap::new())),
            read_handles: Arc::new(Mutex::new(HashMap::new())),
        };
        Ok(Self {
            rt,
            inner: Arc::new(inner),
        })
    }

    fn is_write(flags: i32) -> bool {
        let write_flags = libc::O_RDWR | libc::O_WRONLY;
        (flags & write_flags) > 0
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

        let db = &self.inner.db.clone();
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
        let inner = self.inner.clone();
        let name = name.to_owned();
        self.rt.spawn(async move {
            let name = name.to_owned();
            let node = inner.db.create_node(parent, &name, true).await;
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
        });
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
        // Block creation of Zone.Identifier files from Windows cause that shit's annoying
        if name.to_string_lossy().ends_with("Zone.Identifier") {
            reply.error(EUNKNOWN);
            return;
        }

        let node = self
            .rt
            .block_on(async { self.inner.db.create_node(parent, name, false).await });
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
        let inner = self.inner.clone();
        self.rt.spawn(async move {
            let node = inner.db.get_node_by_id(ino).await;
            match node {
                Ok(n) => match n {
                    Some(n) => {
                        if Self::is_write(flags) {
                            info!(
                                "create file: {}",
                                n.name.as_ref().unwrap_or(&"".to_string())
                            );
                            let file = inner.client.open_file_write(n).await;
                            inner.write_handles.lock().await.insert(ino, file);
                            reply.opened(0, 0);
                        } else {
                            info!("read file: {}", n.name.as_ref().unwrap_or(&"".to_string()));
                            if let Ok(file) = inner.client.open_file_read(n).await {
                                inner.read_handles.lock().await.insert(ino, file);
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
        });
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
        let inner = self.inner.clone();
        let data = data.to_owned();
        self.rt.spawn(async move {
            let mut handles = inner.write_handles.lock().await;
            let file = handles.get_mut(&ino);
            if let Some(handle) = file {
                if let Ok(written) = handle.write(&data).await {
                    reply.written(written as u32)
                } else {
                    reply.error(EUNKNOWN)
                };
            } else {
                reply.error(EUNKNOWN);
            }
        });
    }

    fn getattr(&mut self, _req: &fuser::Request<'_>, ino: u64, reply: fuser::ReplyAttr) {
        let inner = self.inner.clone();
        self.rt.spawn(async move {
            let result = inner.db.get_node_by_id(ino).await;
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
        });
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
        let inner = self.inner.clone();
        self.rt.spawn(async move {
            if Self::is_write(flags) {
                if let Some(handle) = inner.write_handles.lock().await.get_mut(&ino) {
                    match handle.flush().await {
                        Ok(_) => {
                            handle.finish();
                            reply.ok()
                        }
                        Err(_) => reply.error(EUNKNOWN),
                    }
                    inner.write_handles.lock().await.remove(&ino);
                } else {
                    reply.error(ENOENT)
                }
            } else {
                if let Some(handle) = inner.read_handles.lock().await.remove(&ino) {
                    handle.finish();
                    reply.ok();
                } else {
                    reply.error(ENOENT);
                }
            }
        });
    }

    fn readdir(
        &mut self,
        _req: &fuser::Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: fuser::ReplyDirectory,
    ) {
        debug!("readdir ino: {:?} offset: {:?}", ino, offset);
        let inner = self.inner.clone();
        self.rt.spawn(async move {
            let result = inner.db.get_nodes_by_parent(ino as i64).await;
            if let Ok(nodes) = result {
                debug!("files {:?}", &nodes);
                let mut full = false;
                let mut i = offset as usize;
                while !full && i < nodes.len() {
                    let node = &nodes[i];
                    full = reply.add(
                        node.id as u64,
                        (i + 1) as i64,
                        if node.directory {
                            FileType::Directory
                        } else {
                            FileType::RegularFile
                        },
                        node.name.clone().unwrap_or_else(|| "".to_string()),
                    );
                    i += 1;
                }
                reply.ok();
            } else {
                reply.error(EUNKNOWN);
            }
        });
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
        let inner = self.inner.clone();
        self.rt.spawn(async move {
            if let Some(handle) = inner.read_handles.lock().await.get_mut(&ino) {
                let mut buffer = vec![0; size as usize].into_boxed_slice();
                let result = handle.read(&mut buffer).await;
                if let Ok(written) = result {
                    debug!("written: {:?}", written);
                    reply.data(&buffer[..written]);
                } else {
                    reply.error(EUNKNOWN);
                }
            } else {
                reply.error(ENOENT);
            }
        });
    }

    fn rmdir(
        &mut self,
        _req: &fuser::Request<'_>,
        parent: u64,
        name: &std::ffi::OsStr,
        reply: fuser::ReplyEmpty,
    ) {
        let inner = self.inner.clone();
        let name = name.to_owned();
        self.rt.spawn(async move {
            let result = inner
                .db
                .delete_node(parent as i64, &name.to_string_lossy(), true)
                .await;
            if let Ok(deleted) = result {
                if deleted == 0 {
                    reply.error(ENOENT);
                } else {
                    info!("deleted directory: {:?}", name);
                    reply.ok();
                }
            } else {
                reply.error(EUNKNOWN);
            }
        });
    }

    fn unlink(
        &mut self,
        _req: &fuser::Request<'_>,
        parent: u64,
        name: &std::ffi::OsStr,
        reply: fuser::ReplyEmpty,
    ) {
        let inner = self.inner.clone();
        let name = name.to_owned();
        self.rt.spawn(async move {
            let result = inner
                .db
                .delete_node(parent as i64, &name.to_string_lossy(), false)
                .await;
            if let Ok(deleted) = result {
                if deleted == 0 {
                    reply.error(ENOENT);
                } else {
                    info!("deleted file: {:?}", name);
                    reply.ok();
                }
            } else {
                reply.error(EUNKNOWN);
            }
        });
    }

    fn rename(
        &mut self,
        _req: &fuser::Request<'_>,
        parent: u64,
        name: &std::ffi::OsStr,
        newparent: u64,
        newname: &std::ffi::OsStr,
        _flags: u32,
        reply: fuser::ReplyEmpty,
    ) {
        let inner = self.inner.clone();
        let name = name.to_owned();
        let newparent = newparent.to_owned();
        let newname = newname.to_owned();
        self.rt.spawn(async move {
            let result = inner
                .db
                .move_node(
                    parent as i64,
                    &name.to_string_lossy(),
                    newparent as i64,
                    &newname.to_string_lossy(),
                )
                .await;
            match result {
                Ok(_) => reply.ok(),
                Err(DbError::DoesNotExist(_)) => reply.error(ENOENT),
                Err(_) => reply.error(EUNKNOWN),
            }
        });
    }
}

pub enum CloudType {
    Discord,
}

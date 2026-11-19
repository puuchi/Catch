use crate::Result;
use atomicwrites::{AllowOverwrite, AtomicFile};
use fs2::FileExt;
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Component, Path, PathBuf},
};

/// Small fault-injection boundary shared by all JSON repositories.
pub trait Disk: Send + Sync {
    fn read(&self, name: &str) -> Result<Option<Vec<u8>>>;
    fn write_atomic(&self, name: &str, bytes: &[u8]) -> Result<()>;
    fn remove(&self, name: &str) -> Result<()>;
}
pub struct FileDisk {
    directory: PathBuf,
}
impl FileDisk {
    pub fn new(directory: impl Into<PathBuf>) -> Result<Self> {
        let directory = directory.into();
        fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
        Ok(Self { directory })
    }
    fn path(&self, name: &str) -> Result<PathBuf> {
        let parts: Vec<_> = Path::new(name).components().collect();
        if parts.len() != 1
            || !matches!(parts[0], Component::Normal(_))
            || name.contains(['/', '\\', ':'])
        {
            return Err("存储文件名无效".into());
        }
        Ok(self.directory.join(name))
    }
}
impl Disk for FileDisk {
    fn read(&self, name: &str) -> Result<Option<Vec<u8>>> {
        match fs::read(self.path(name)?) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }
    fn write_atomic(&self, name: &str, bytes: &[u8]) -> Result<()> {
        AtomicFile::new(self.path(name)?, AllowOverwrite)
            .write(|file| {
                file.write_all(bytes)?;
                file.sync_all()
            })
            .map_err(|error| format!("保存失败：{error}"))
    }
    fn remove(&self, name: &str) -> Result<()> {
        fs::remove_file(self.path(name)?).map_err(|error| format!("移除恢复日志失败：{error}"))
    }
}

/// A persistent lock file avoids the unlink/recreate race of PID lock files.
/// Windows share denial additionally prevents a second runtime from opening it.
/// The legacy Electron adapter must participate before a real profile is used.
pub struct ProfileLock {
    _file: File,
}
impl ProfileLock {
    pub fn acquire(directory: &Path) -> Result<Self> {
        fs::create_dir_all(directory).map_err(|error| error.to_string())?;
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.share_mode(0);
        }
        let file = options
            .open(directory.join(".catch-write.lock"))
            .map_err(|error| format!("数据目录正在使用或无法锁定：{error}"))?;
        file.try_lock_exclusive()
            .map_err(|error| format!("数据目录正在使用：{error}"))?;
        Ok(Self { _file: file })
    }
}

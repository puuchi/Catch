//! JSON persistence and recovery. No Tauri or renderer dependency.
//! The desktop composition root must retain ProfileLock for the whole lifetime
//! of all stores using the same directory.
mod disk;
mod growth;
mod schedule;
mod todos;
mod tracking;
pub use disk::{Disk, FileDisk, ProfileLock};
pub use growth::GrowthStore;
pub use todos::TodosStore;
pub use tracking::TrackingStore;

pub type Result<T> = std::result::Result<T, String>;
pub(crate) fn encode<T: serde::Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_json::to_vec_pretty(value).map_err(|error| format!("数据编码失败：{error}"))
}

/// Global Sled Instance
/// Single shared sled database to prevent malloc_consolidate crashes
/// Each module gets unique trees within this DB
///
/// CRITICAL: This DB is intentionally LEAKED to prevent Drop from running.
/// Sled has background flush threads. If Drop runs during process exit,
/// those threads race with heap deallocation → malloc_consolidate() crash.
/// 
/// Leaking is safe here:
/// - Sled flushes data on write, not just on drop
/// - OS cleans up /tmp files on reboot
/// - Tests get clean state via unique tree names

use sled::Db;
use std::mem::ManuallyDrop;
use std::ops::Deref;

pub struct NoDropDb(ManuallyDrop<Db>);

impl Deref for NoDropDb {
    type Target = Db;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

unsafe impl Send for NoDropDb {}
unsafe impl Sync for NoDropDb {}

lazy_static::lazy_static! {
    pub static ref GLOBAL_SLED_DB: NoDropDb = {
        let db = sled::open("/tmp/lapce_global_sled_db")
            .expect("Failed to open global sled DB");
        NoDropDb(ManuallyDrop::new(db))
    };
}

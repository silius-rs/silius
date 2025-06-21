use silius_storage::db::SiliusDB;

pub struct UserOperationPool {
    pub db: SiliusDB,
}

impl UserOperationPool {
    pub fn new(db: SiliusDB) -> Self {
        Self { db }
    }
}

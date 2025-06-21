use silius_storage::db::SiliusDB;

pub struct Reputation {
    pub db: SiliusDB,
}

impl Reputation {
    pub fn new(db: SiliusDB) -> Self {
        Self { db }
    }
}
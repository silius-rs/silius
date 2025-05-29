use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Redb error: {0}")]
    Redb(#[from] Box<redb::Error>),

    #[error("Io error in creating DB file: {0}")]
    Io(#[from] std::io::Error),

    #[error("Field not initilized")]
    FieldNotInitilized,
}

impl From<redb::Error> for DatabaseError {
    fn from(err: redb::Error) -> Self {
        DatabaseError::Redb(Box::new(err))
    }
}

impl From<redb::TransactionError> for DatabaseError {
    fn from(err: redb::TransactionError) -> Self {
        DatabaseError::Redb(Box::new(err.into()))
    }
}

impl From<redb::TableError> for DatabaseError {
    fn from(err: redb::TableError) -> Self {
        DatabaseError::Redb(Box::new(err.into()))
    }
}

impl From<redb::CommitError> for DatabaseError {
    fn from(err: redb::CommitError) -> Self {
        DatabaseError::Redb(Box::new(err.into()))
    }
}

impl From<redb::StorageError> for DatabaseError {
    fn from(err: redb::StorageError) -> Self {
        DatabaseError::Redb(Box::new(err.into()))
    }
}

impl From<redb::DatabaseError> for DatabaseError {
    fn from(err: redb::DatabaseError) -> Self {
        DatabaseError::Redb(Box::new(err.into()))
    }
}

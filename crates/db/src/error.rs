use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("Invalid iterator order {value}, must be 1 (asc) or 2 (desc)")]
    InvalidOrder {
        value: i32,
    },

    #[error("Cannot find iterator with id `{iterator_id}`")]
    IteratorNotFound {
        iterator_id: i32,
    },

    #[error("Failed to disassemble SharedStore because more than 1 strong references remains")]
    StillReferenced,
}

pub type DbResult<T> = std::result::Result<T, DbError>;

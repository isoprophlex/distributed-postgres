use postgres::error::Error;
use std::fmt;

/// SQLSTATE code error for "relation does not exist"
const UNDEFINED_TABLE_CODE: &str = "42P01";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendQueryError {
    ClientIsClosed,
    UndefinedTable,
    Other(String),
}

/// Implementation of Display for SendQueryError
impl fmt::Display for SendQueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SendQueryError::ClientIsClosed => write!(f, "Client is closed"),
            SendQueryError::UndefinedTable => write!(f, "Table is not defined"),
            SendQueryError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for SendQueryError {}

/// Checks if the error is a connection closed error
pub fn is_connection_closed(err: &Error) -> bool {
    let err = format!("{}", err);
    return err.contains("connection closed") || err.contains("kind: Connection reset by peer");
}

/// Checks if the error is an undefined table error
pub fn is_undefined_table(err: &Error) -> bool {
    if let Some(db_error) = err.as_db_error() {
        return db_error.code().code() == UNDEFINED_TABLE_CODE;
    }
    return false;
}

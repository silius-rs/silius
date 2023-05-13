use std::error::Error;
use std::fmt::{Display, Formatter, Result};

#[derive(Debug)]
pub enum UoPoolError {
    InvalidUO,
    InvalidEntryPoint,
    ErrorAddingUO,
}

impl Error for UoPoolError {}

impl Display for UoPoolError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            UoPoolError::InvalidUO => {
                write!(f, "Invalid UO")
            }
            UoPoolError::InvalidEntryPoint => {
                write!(f, "Invalid entry point")
            }
            UoPoolError::ErrorAddingUO => {
                write!(f, "Error adding UO")
            }
        }
    }
}

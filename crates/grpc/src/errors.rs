use thiserror::Error;
use tonic::{Code, Status};

#[derive(Debug, Error)]
pub enum GrpcErrors {
    #[error("Server aborted the request: {0}")]
    Aborted(String),
    #[error("Bundle already exists: {0}")]
    BundleExists(String),
    #[error("Timeout, deadline exceeded: {0}")]
    DeadlineExceeded(String),
    #[error("Invalid argument provided: {0}")]
    InvalidArgument(String),
    #[error("Bad GRPC Status Code received: {0}")]
    BadGrpcStatusCode(String),
    #[error("Unknown error: {0}")]
    Unknown(String),
    #[error("User Operation Missiong: {0}")]
    UserOperationMissing(String),
}

impl From<Status> for GrpcErrors {
    fn from(status: Status) -> Self {
        match status.code() {
            Code::Aborted => GrpcErrors::Aborted(status.message().to_string()),
            Code::DeadlineExceeded => GrpcErrors::DeadlineExceeded(status.message().to_string()),
            Code::InvalidArgument => GrpcErrors::InvalidArgument(status.message().to_string()),
            Code::AlreadyExists => GrpcErrors::BundleExists(status.message().to_string()),
            Code::NotFound => GrpcErrors::UserOperationMissing(status.message().to_string()),
            Code::Unknown => GrpcErrors::Unknown(status.message().to_string()),
            Code::OutOfRange => GrpcErrors::BadGrpcStatusCode(status.message().to_string()),
            _ => GrpcErrors::Aborted(status.message().to_string()),
        }
    }
}

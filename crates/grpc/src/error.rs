use tonic::Status;

#[derive(Debug, Display, Error, Clone)]
pub enum GrpcErrors {
    Aborted(String),
    AlreadyExists(String),
    Cancelled(String),
    DataLoss(String),
    DeadlineExceeded(String),
    FailedPrecondition(String),
    InvalidArgument(String),
    Internal(String),
    NotFound(String),
    OutOfRange(String),
    PermissionDenied(String),
    ResourceExhausted(String),
    Unauthenticated(String),
    Unavailable(String),
    Unimplemented(String),
    Unknown(String),
}

impl From<Status> for GrpcErrors {
   fn from(status: Status) -> Self {
        match status.code() {
            Code::Aborted => GrpcErrors::Aborted(status.message().to_string()),
            Code::AlreadyExists => GrpcErrors::AlreadyExists(status.message().to_string()),
            Code::Cancelled => GrpcErrors::Cancelled(status.message().to_string()),
            Code::DataLoss => GrpcErrors::DataLoss(status.message().to_string()),
            Code::DeadlineExceeded => GrpcErrors::DeadlineExceeded(status.message().to_string()),
            Code::FailedPrecondition => GrpcErrors::FailedPrecondition(status.message().to_string()),
            Code::InvalidArgument => GrpcErrors::InvalidArgument(status.message().to_string()),
            Code::Internal => GrpcErrors::Internal(status.message().to_string()),
            Code::NotFound => GrpcErrors::NotFound(status.message().to_string()),
            Code::OutOfRange => GrpcErrors::OutOfRange(status.message().to_string()),
            Code::PermissionDenied => GrpcErrors::PermissionDenied(status.message().to_string()),
            Code::ResourceExhausted => GrpcErrors::ResourceExhausted(status.message().to_string()),
            Code::Unauthenticated => GrpcErrors::Unauthenticated(status.message().to_string()),
            Code::Unavailable => GrpcError::Unavailable(status.message().to_string()),
            Code::Unimplemented => GrpcError::Unimplemented(status.message().to_string()),
            Code::Unknown => GrpcError::Unknown(status.message().to_string()),
        }
   }
}
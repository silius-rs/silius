use actix_web::{HttpResponse, Responder, get};
use serde::{Deserialize, Serialize};
use silius_node_version::silius_node_version;

use crate::types::{error::ApiError, response::DataResponse};

#[derive(Serialize, Deserialize, Default)]
pub struct Version {
    version: String,
}

impl Version {
    pub fn new() -> Self {
        Self {
            version: silius_node_version(),
        }
    }
}

#[get("/node/version")]
pub async fn client_version() -> Result<impl Responder, ApiError> {
    Ok(HttpResponse::Ok().json(DataResponse::new(Version::new())))
}

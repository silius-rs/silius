use actix_web::web::ServiceConfig;

use crate::handlers::version::client_version;

pub fn register_web3_routes(cfg: &mut ServiceConfig) {
    cfg.service(client_version);
}

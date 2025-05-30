use actix_web::web::ServiceConfig;

pub fn register_eth_routes(cfg: &mut ServiceConfig) {
    cfg.service(send_user_operation)
        .service(estimate_user_operation_gas)
        .service(get_user_operation_by_hash)
        .service(get_user_operation_receipt)
        .service(supported_entry_points)
        .service(chain_id);
}

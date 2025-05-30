use actix_web::web::ServiceConfig;

pub fn register_debug_routes(cfg: &mut ServiceConfig) {
    cfg.service(clear_state)
        .service(dump_mempool)
        .service(send_bundle_now)
        .service(set_bundling_mode)
        .service(set_reputation)
        .service(dump_reputation)
        .service(add_user_ops);
}

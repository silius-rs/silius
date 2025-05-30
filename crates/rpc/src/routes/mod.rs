use actix_web::web::{ServiceConfig, scope};

pub mod debug;
pub mod eth;
pub mod web3;

pub fn register_routers(config: &mut ServiceConfig) {
    config
        .service(scope("/web3").configure(web3::register_web3_routes))
        .service(scope("/eth").configure(eth::register_eth_routes))
        .service(scope("/debug").configure(debug::register_debug_routes));
}

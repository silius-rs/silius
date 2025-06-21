use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use actix_web::{
    App, Error, HttpResponse, HttpServer, middleware,
    web::{self, Bytes, Data},
};
use alloy_provider::Provider;
use silius_manager::SiliusManager;
use tracing::info;

use crate::{RPC_PATH, StopHandle, routes::rpc_router};

const CONTENT_TYPE: &str = "application/json";

#[derive(Debug, Clone)]
pub struct HttpRpcServerConfig {
    pub http_socket_address: SocketAddr,
    pub http_allow_origins: Vec<String>,
    pub http_api_modules: Vec<String>,
}

impl HttpRpcServerConfig {
    pub fn new(
        http_address: IpAddr,
        http_port: u16,
        http_allow_origins: Vec<String>,
        http_api_modules: Vec<String>,
    ) -> Self {
        Self {
            http_socket_address: SocketAddr::new(http_address, http_port),
            http_allow_origins,
            http_api_modules,
        }
    }
}

pub async fn http_handler<P: Provider + Clone + 'static>(
    body: Bytes,
    api_modules: Data<Vec<String>>,
    manager: Data<Arc<SiliusManager<P>>>,
) -> Result<HttpResponse, Error> {
    let response = rpc_router(body, api_modules, manager).await?;
    Ok(HttpResponse::Ok().content_type(CONTENT_TYPE).body(response))
}

pub async fn start_http_server<P: Provider + Clone + 'static>(
    server_config: HttpRpcServerConfig,
    manager: Arc<SiliusManager<P>>,
) -> std::io::Result<()> {
    info!(
        "Starting HTTP server on: {}",
        server_config.http_socket_address
    );
    info!("Enabled API modules: {:?}", server_config.http_api_modules);
    let stop_handle = Data::new(StopHandle::default());

    let server = HttpServer::new(move || {
        let stop_handle = stop_handle.clone();
        App::new()
            .wrap(middleware::Logger::default())
            .app_data(stop_handle)
            .app_data(Data::new(server_config.http_api_modules.clone()))
            .app_data(Data::new(manager.clone()))
            .service(web::resource(RPC_PATH).route(web::post().to(http_handler::<P>)))
    })
    .bind(server_config.http_socket_address)?
    .run();

    server.await
}

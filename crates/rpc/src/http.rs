use std::net::{IpAddr, SocketAddr};

use actix_web::{
    App, Error, HttpResponse, HttpServer, middleware,
    web::{self, Bytes, Data},
};
use silius_storage::db::SiliusDB;
use tracing::info;

use crate::{RPC_PATH, StopHandle, routes::rpc_router};

const CONTENT_TYPE: &str = "application/json";

#[derive(Debug, Clone)]
pub struct HttpRpcServerConfig {
    pub http_socket_address: SocketAddr,
    pub http_allow_origins: Vec<String>,
}

impl HttpRpcServerConfig {
    pub fn new(http_address: IpAddr, http_port: u16, http_allow_origins: Vec<String>) -> Self {
        Self {
            http_socket_address: SocketAddr::new(http_address, http_port),
            http_allow_origins,
        }
    }
}

pub async fn http_handler(body: Bytes, db: Data<SiliusDB>) -> Result<HttpResponse, Error> {
    let response = rpc_router(body, db).await?;
    Ok(HttpResponse::Ok().content_type(CONTENT_TYPE).body(response))
}

pub async fn start_http_server(
    server_config: HttpRpcServerConfig,
    db: SiliusDB,
) -> std::io::Result<()> {
    info!(
        "Starting HTTP server on {}",
        server_config.http_socket_address
    );
    let stop_handle = Data::new(StopHandle::default());

    let server = HttpServer::new(move || {
        let stop_handle = stop_handle.clone();
        App::new()
            .wrap(middleware::Logger::default())
            .app_data(stop_handle)
            .app_data(Data::new(db.clone()))
            .service(web::resource(RPC_PATH).route(web::post().to(http_handler)))
    })
    .bind(server_config.http_socket_address)?
    .run();

    server.await
}

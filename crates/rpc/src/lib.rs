use actix_web::{
    App, HttpServer,
    dev::ServerHandle,
    middleware,
    web::{self, Data},
};
use config::HttpRpcServerConfig;
use routes::rpc_router;
use silius_storage::db::SiliusDB;
use tracing::info;

pub mod config;
pub mod handlers;
pub mod routes;
pub mod types;

pub const RPC_PATH: &str = "/rpc";

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
            .service(web::resource(RPC_PATH).route(web::post().to(rpc_router)))
    })
    .bind(server_config.http_socket_address)?
    .run();

    server.await
}

#[derive(Default)]
struct StopHandle {
    inner: parking_lot::Mutex<Option<ServerHandle>>,
}

#[allow(dead_code)]
impl StopHandle {
    /// Sets the server handle to stop.
    pub(crate) fn register(&self, handle: ServerHandle) {
        *self.inner.lock() = Some(handle);
    }

    /// Sends stop signal through contained server handle.
    pub(crate) fn stop(&self, graceful: bool) {
        #[allow(clippy::let_underscore_future)]
        let _ = self.inner.lock().as_ref().unwrap().stop(graceful);
    }
}

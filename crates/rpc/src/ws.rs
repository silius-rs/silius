use std::net::{IpAddr, SocketAddr};

use actix_web::{
    App, Error, HttpRequest, HttpResponse, HttpServer, middleware, rt,
    web::{self, Data},
};
use actix_ws::{AggregatedMessage, handle};
use futures_util::StreamExt as _;
use silius_storage::db::SiliusDB;
use tracing::{debug, info};

use crate::{RPC_PATH, StopHandle, routes::rpc_router};

#[derive(Debug, Clone)]
pub struct WsRpcServerConfig {
    pub ws_socket_address: SocketAddr,
    pub ws_allow_origins: Vec<String>,
}

impl WsRpcServerConfig {
    pub fn new(ws_address: IpAddr, ws_port: u16, ws_allow_origins: Vec<String>) -> Self {
        Self {
            ws_socket_address: SocketAddr::new(ws_address, ws_port),
            ws_allow_origins,
        }
    }
}

pub async fn ws_handler(
    request: HttpRequest,
    stream: web::Payload,
    db: Data<SiliusDB>,
) -> Result<HttpResponse, Error> {
    let (res, mut session, stream) = handle(&request, stream)?;

    let mut stream = stream
        .aggregate_continuations()
        .max_continuation_size(2_usize.pow(20));

    rt::spawn(async move {
        while let Some(msg) = stream.next().await {
            match msg {
                Ok(AggregatedMessage::Text(text)) => {
                    match rpc_router(text.into_bytes(), db.clone()).await {
                        Ok(response) => {
                            if let Err(e) =
                                session.text(std::str::from_utf8(&response).unwrap()).await
                            {
                                debug!("Error sending response: {}", e);
                                break;
                            }
                        }
                        Err(e) => {
                            debug!("Error processing text message: {}", e);
                        }
                    }
                }
                Ok(AggregatedMessage::Binary(bin)) => match rpc_router(bin, db.clone()).await {
                    Ok(response) => {
                        if let Err(e) = session.binary(response).await {
                            debug!("Error sending response: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        debug!("Error processing binary message: {}", e);
                    }
                },
                Ok(AggregatedMessage::Ping(msg)) => {
                    if let Err(e) = session.pong(&msg).await {
                        debug!("Error responding to ping message: {}", e);
                        break;
                    }
                }
                _ => {}
            }
        }
    });

    Ok(res)
}

pub async fn start_ws_server(
    server_config: WsRpcServerConfig,
    db: SiliusDB,
) -> std::io::Result<()> {
    info!("Starting WS server on {}", server_config.ws_socket_address);
    let stop_handle = Data::new(StopHandle::default());

    let server = HttpServer::new(move || {
        let stop_handle = stop_handle.clone();
        App::new()
            .wrap(middleware::Logger::default())
            .app_data(stop_handle)
            .app_data(Data::new(db.clone()))
            .service(web::resource(RPC_PATH).route(web::get().to(ws_handler)))
    })
    .bind(server_config.ws_socket_address)?
    .run();

    server.await
}

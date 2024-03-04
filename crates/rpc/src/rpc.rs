use super::middleware::ProxyJsonRpcLayer;
use eyre::Error;
use hyper::{http::HeaderValue, Method};
use jsonrpsee::{
    server::{RpcServiceBuilder, ServerBuilder, ServerHandle},
    Methods,
};
use silius_metrics::rpc::MetricsLayer;
use std::net::{IpAddr, SocketAddr};
use tower::ServiceBuilder;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};

/// JsonRpcServer is a wrapper around the `jsonrpsee` [ServerBuilder](https://docs.rs/jsonrpsee/3.0.0-beta.1/jsonrpsee/server/struct.ServerBuilder.html).
pub struct JsonRpcServer {
    /// Whether to start an HTTP server.
    http: bool,
    /// HTTP address to listen on.
    http_addr: IpAddr,
    /// HTTP port to listen on.
    http_port: u16,
    /// The HTTP RPC methods to be exposed.
    http_methods: Methods,
    /// The [cors layer](CorsLayer) for HTTP server to filter requests.
    http_cors_layer: Option<CorsLayer>,
    /// Whether to start a WS server.
    ws: bool,
    /// WS address to listen on.
    ws_addr: IpAddr,
    /// WS port to listen on.
    ws_port: u16,
    /// The WS RPC methods to be exposed.
    ws_methods: Methods,
    /// The [cors layer](CorsLayer) for WS server to filter requests.   
    ws_cors_layer: Option<CorsLayer>,
    /// The [proxy layer](ProxyJsonRpcLayer) to forward requests.
    proxy_layer: Option<ProxyJsonRpcLayer>,
    /// This [metric layer](MetricsLayer) is used for collecting and reporting metrics related to
    /// RPC operations.
    metric_layer: Option<MetricsLayer>,
}

pub enum JsonRpcServerType {
    /// Both HTTP and WS.
    Both,
    /// Only HTTP.
    Http,
    /// Only WS.
    Ws,
}

impl JsonRpcServer {
    /// Create a new JsonRpcServer.
    ///
    /// # Arguments
    /// * `http: bool` - Whether to start an HTTP server.
    /// * `http_addr: IpAddr` - HTTP address to listen on.
    /// * `http_port: u16` - HTTP port to listen on.
    /// * `ws: bool` - Whether to start a WS server.
    /// * `ws_addr: IpAddr` - WS address to listen on.
    /// * `ws_port: u16` - WS port to listen on.
    ///
    /// # Returns
    /// * `Self` - A new [JsonRpcServer](JsonRpcServer) instance.
    pub fn new(
        http: bool,
        http_addr: IpAddr,
        http_port: u16,
        ws: bool,
        ws_addr: IpAddr,
        ws_port: u16,
    ) -> Self {
        Self {
            http,
            http_addr,
            http_port,
            http_methods: Methods::new(),
            http_cors_layer: None,
            ws,
            ws_addr,
            ws_port,
            ws_methods: Methods::new(),
            ws_cors_layer: None,
            proxy_layer: None,
            metric_layer: None,
        }
    }

    /// Add a cors layer to the server.
    ///
    /// # Arguments
    /// * `cors_domain: Vec<String>` - A list of CORS filters in the form of String.
    /// * `typ: JsonRpcServerType` - The type of the server.
    ///
    /// # Returns
    /// * `Self` - A new [JsonRpcServer](JsonRpcServer) instance.
    pub fn with_cors(mut self, cors_domain: &[String], typ: JsonRpcServerType) -> Self {
        let cors_layer = if cors_domain.iter().any(|d| d == "*") {
            CorsLayer::new().allow_headers(Any).allow_methods([Method::POST]).allow_origin(Any)
        } else {
            let mut origins: Vec<HeaderValue> = vec![];

            for domain in cors_domain.iter() {
                if let Ok(origin) = domain.parse::<HeaderValue>() {
                    origins.push(origin);
                }
            }

            CorsLayer::new()
                .allow_headers(Any)
                .allow_methods([Method::POST])
                .allow_origin(AllowOrigin::list(origins))
        };

        match typ {
            JsonRpcServerType::Both => {
                self.http_cors_layer = Some(cors_layer.clone());
                self.ws_cors_layer = Some(cors_layer);
            }
            JsonRpcServerType::Http => self.http_cors_layer = Some(cors_layer),
            JsonRpcServerType::Ws => self.ws_cors_layer = Some(cors_layer),
        }

        self
    }

    /// Add a proxy layer to the server.
    ///
    /// # Arguments
    /// * `eth_client_address: String` - The address of the Ethereum execution client.
    ///
    /// # Returns
    /// * `Self` - The JsonRpcServer instance.
    pub fn with_proxy(mut self, eth_client_address: String) -> Self {
        self.proxy_layer = Some(ProxyJsonRpcLayer::new(eth_client_address));
        self
    }

    pub fn with_metrics(mut self) -> Self {
        self.metric_layer = Some(MetricsLayer::new());
        self
    }

    /// Add methods to the RPC server.
    ///
    /// # Arguments
    /// * `methods: impl Into<Methods>` - The RPC methods to be exposed.
    /// * `typ: JsonRpcServerType` - The type of the server.
    ///
    /// # Returns
    /// * `Result<(), Error>` - None if no error.
    pub fn add_methods(
        &mut self,
        methods: impl Into<Methods>,
        typ: JsonRpcServerType,
    ) -> Result<(), Error> {
        let methods: Methods = methods.into();

        match typ {
            JsonRpcServerType::Both => {
                self.http_methods.merge(methods.clone())?;
                self.ws_methods.merge(methods).map_err(|e| e.into())
            }
            JsonRpcServerType::Http => self.http_methods.merge(methods).map_err(|e| e.into()),
            JsonRpcServerType::Ws => self.ws_methods.merge(methods).map_err(|e| e.into()),
        }
    }

    /// Starts the JSON-RPC server.
    ///
    /// # Returns
    /// * `Result<(Option<ServerHandle>, Option<ServerHandle>), Error>` - The
    ///   [ServerHandle]((`Option<ServerHandle>`, `Option<ServerHandle>`)) of the HTTP and WS
    ///   servers.
    pub async fn start(&self) -> eyre::Result<(Option<ServerHandle>, Option<ServerHandle>)> {
        let http_handle = if self.http {
            let service = ServiceBuilder::new()
                .option_layer(self.http_cors_layer.clone())
                .option_layer(self.proxy_layer.clone());
            let rpc_service = RpcServiceBuilder::new().option_layer(self.metric_layer.clone());

            let server = ServerBuilder::new()
                .http_only()
                .set_rpc_middleware(rpc_service)
                .set_http_middleware(service)
                .build(SocketAddr::new(self.http_addr, self.http_port))
                .await?;

            Some(server.start(self.http_methods.clone()))
        } else {
            None
        };
        let ws_handle = if self.ws {
            let service = ServiceBuilder::new()
                .option_layer(self.ws_cors_layer.clone())
                .option_layer(self.proxy_layer.clone());
            let rpc_service = RpcServiceBuilder::new().option_layer(self.metric_layer.clone());
            let server = ServerBuilder::new()
                .ws_only()
                .set_rpc_middleware(rpc_service)
                .set_http_middleware(service)
                .build(SocketAddr::new(self.ws_addr, self.ws_port))
                .await?;

            Some(server.start(self.ws_methods.clone()))
        } else {
            None
        };

        Ok((http_handle, ws_handle))
    }
}

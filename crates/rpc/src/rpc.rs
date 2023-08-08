use super::middleware::ProxyJsonRpcLayer;
use anyhow::Error;
use hyper::{http::HeaderValue, Method};
use jsonrpsee::{
    server::{ServerBuilder, ServerHandle},
    Methods,
};
use tower::ServiceBuilder;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};

/// JsonRpcServer is a wrapper around the `jsonrpsee` [ServerBuilder](https://docs.rs/jsonrpsee/3.0.0-beta.1/jsonrpsee/server/struct.ServerBuilder.html).
pub struct JsonRpcServer {
    /// The address to listen on.
    listen_address: String,
    /// The [cors layer](CorsLayer) to filter requests.
    cors_layer: Option<CorsLayer>,
    /// The [proxy layer](ProxyJsonRpcLayer) to forward requests.
    proxy_layer: Option<ProxyJsonRpcLayer>,
    /// The RPC methods to be exposed.
    methods: Methods,
}

impl JsonRpcServer {
    /// Create a new JsonRpcServer.
    ///
    /// # Arguments
    /// * `listen_address: String` - The address to listen on.
    ///
    /// # Returns
    /// * `Self` - A new [JsonRpcServer](JsonRpcServer) instance.
    pub fn new(listen_address: String) -> Self {
        Self {
            listen_address,
            cors_layer: None,
            proxy_layer: None,
            methods: Methods::new(),
        }
    }

    /// Add a cors layer to the server.
    ///
    /// # Arguments
    /// * `cors_domain: Vec<String>` - A list of CORS filters in the form of String.
    ///
    /// # Returns
    /// * `Self` - A new [JsonRpcServer](JsonRpcServer) instance.
    pub fn with_cors(mut self, cors_domain: Vec<String>) -> Self {
        let cors_layer = if cors_domain.iter().any(|d| d == "*") {
            CorsLayer::new()
                .allow_headers(Any)
                .allow_methods([Method::POST])
                .allow_origin(Any)
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

        self.cors_layer = Some(cors_layer);
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

    /// Add a method to the RPC server.
    ///
    /// # Arguments
    /// * `methods: impl Into<Methods>` - The RPC methods to be exposed.
    ///
    /// # Returns
    /// * `Result<(), Error>` - None if no error.
    pub fn add_method(&mut self, methods: impl Into<Methods>) -> Result<(), Error> {
        self.methods.merge(methods).map_err(|e| e.into())
    }

    /// Start the [json RPC server](JsonRpcServer)
    ///
    /// # Returns
    /// * `Result<ServerHandle, Error>` - The [handle](ServerHandle) of the server.
    pub async fn start(&self) -> anyhow::Result<ServerHandle> {
        let service = ServiceBuilder::new()
            .option_layer(self.cors_layer.clone())
            .option_layer(self.proxy_layer.clone());

        let server = ServerBuilder::new()
            .set_middleware(service)
            .build(&self.listen_address)
            .await?;

        Ok(server.start(self.methods.clone())?)
    }
}

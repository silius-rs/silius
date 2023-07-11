use super::middleware::ProxyJsonRpcLayer;
use anyhow::Error;
use hyper::{http::HeaderValue, Method};
use jsonrpsee::{
    server::{ServerBuilder, ServerHandle},
    Methods,
};
use tower::ServiceBuilder;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};

pub struct JsonRpcServer {
    listen_address: String,
    cors_layer: Option<CorsLayer>,
    proxy_layer: Option<ProxyJsonRpcLayer>,
    methods: Methods,
}

impl JsonRpcServer {
    pub fn new(listen_address: String) -> Self {
        Self {
            listen_address,
            cors_layer: None,
            proxy_layer: None,
            methods: Methods::new(),
        }
    }

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

    pub fn with_proxy(mut self, eth_client_address: String) -> Self {
        self.proxy_layer = Some(ProxyJsonRpcLayer::new(eth_client_address));
        self
    }

    pub fn add_method(&mut self, methods: impl Into<Methods>) -> Result<(), Error> {
        self.methods.merge(methods).map_err(|e| e.into())
    }

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

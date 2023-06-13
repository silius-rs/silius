use hyper::{Body, Request, Response};
use jsonrpsee::core::error::Error as JsonRpcError;
use jsonrpsee::types::error::{ErrorCode, METHOD_NOT_FOUND_MSG};
use jsonrpsee::types::ErrorObjectOwned;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Poll;
use std::{error::Error, task::Context};
use tower::{Layer, Service};

#[derive(Clone, Debug)]
pub struct ProxyJsonRpcLayer {
    pub address: String,
}

impl ProxyJsonRpcLayer {
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            address: address.into(),
        }
    }
}

impl<S> Layer<S> for ProxyJsonRpcLayer {
    type Service = ProxyJsonRpcRequest<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ProxyJsonRpcRequest::new(inner, &self.address)
            .expect("Should be able to create ProxyJsonRpcRequest")
    }
}

#[derive(Debug, Clone)]
pub struct ProxyJsonRpcRequest<S> {
    inner: S,
    address: Arc<str>,
}

impl<S> ProxyJsonRpcRequest<S> {
    pub fn new(inner: S, address: &str) -> Result<Self, JsonRpcError> {
        Ok(Self {
            inner,
            address: Arc::from(address),
        })
    }
}

impl<S> Service<Request<Body>> for ProxyJsonRpcRequest<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Response: 'static,
    S::Error: Into<Box<dyn Error + Send + Sync>> + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = Box<dyn Error + Send + Sync + 'static>;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    #[inline]
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let addr = String::from(self.address.as_ref());
        let mut inner = self.inner.clone();

        let res_fut = async move {
            let (req_h, req_b) = req.into_parts();
            let req_bb = hyper::body::to_bytes(req_b).await?;
            let fut = inner.call(Request::from_parts(req_h, Body::from(req_bb.clone())));

            let res = fut.await.map_err(|err| err.into())?;

            let (res_h, res_b) = res.into_parts();
            let res_bb = hyper::body::to_bytes(res_b).await?;

            #[derive(serde::Deserialize, Debug)]
            struct JsonRpcErrorResponse {
                error: ErrorObjectOwned,
            }

            if let Ok(err) = serde_json::from_slice::<JsonRpcErrorResponse>(&res_bb) {
                if err.error.code() == ErrorCode::MethodNotFound.code()
                    && err.error.message() == METHOD_NOT_FOUND_MSG
                {
                    let client = hyper::Client::new();
                    let req = Request::post(addr)
                        .header(hyper::header::CONTENT_TYPE, "application/json")
                        .body(Body::from(req_bb))?;
                    let res = client.request(req).await?;
                    return Ok(res);
                }
            }

            Ok(Response::from_parts(res_h, Body::from(res_bb)))
        };

        Box::pin(res_fut)
    }
}

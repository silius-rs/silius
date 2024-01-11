use futures::Future;
use hyper::{Body, Request};
use metrics::{counter, describe_counter};
use std::{error::Error, pin::Pin};
use tower::{Layer, Service};

const GRPC_REQUEST: &str = "silius_grpc_request";
const GRPC_REQUEST_SUCCESS: &str = "silius_grpc_request_success";
const GRPC_REQUEST_FAILED: &str = "silius_grpc_request_failed";

#[derive(Clone, Default)]
pub struct MetricsLayer;

impl<S> Layer<S> for MetricsLayer {
    type Service = MetricService<S>;
    fn layer(&self, inner: S) -> Self::Service {
        MetricService { inner }
    }
}

#[derive(Clone)]
pub struct MetricService<S> {
    inner: S,
}

impl<S> Service<Request<Body>> for MetricService<S>
where
    S: Service<Request<Body>> + Clone + Send + 'static,
    S::Response: 'static,
    S::Error: Into<Box<dyn Error + Send + Sync>> + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let clone = self.inner.clone();
        // take the service that was ready
        let mut inner = std::mem::replace(&mut self.inner, clone);
        let fut = async move {
            let (req_header, body) = req.into_parts();
            let path = req_header.uri.path().to_string();

            counter!(GRPC_REQUEST, "path" => path.clone()).increment(1);
            let result = inner.call(Request::from_parts(req_header, body)).await;
            match result {
                Ok(response) => {
                    counter!(GRPC_REQUEST_SUCCESS, "path" => path.clone()).increment(1);
                    Ok(response)
                }
                Err(e) => {
                    counter!(GRPC_REQUEST_FAILED, "path" => path.clone()).increment(1);
                    Err(e)
                }
            }
        };
        Box::pin(fut)
    }
}

pub fn describe_grpc_metrics() {
    describe_counter!(GRPC_REQUEST, "grpc request count");
    describe_counter!(GRPC_REQUEST_SUCCESS, "grpc request success count");
    describe_counter!(GRPC_REQUEST_FAILED, "grpc request failed count");
}

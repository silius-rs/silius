use jsonrpsee::{
    helpers::MethodResponseResult, server::middleware::rpc::RpcServiceT, types::Request,
    MethodResponse,
};
use metrics::{counter, describe_counter};
use pin_project::pin_project;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tower::Layer;

const RPC_REQUEST: &str = "silius_rpc_request";
const RPC_REQUEST_SUCCESS: &str = "silius_rpc_request_success";
const RPC_REQUEST_FAILED: &str = "silius_rpc_request_failed";

#[derive(Clone, Debug, Default)]
pub struct MetricsLayer;

impl MetricsLayer {
    pub fn new() -> Self {
        Self
    }
}

impl<S> Layer<S> for MetricsLayer {
    type Service = MetricService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        MetricService::new(inner)
    }
}

#[derive(Clone)]
pub struct MetricService<T> {
    inner: T,
}

impl<T> MetricService<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<'a, S> RpcServiceT<'a> for MetricService<S>
where
    S: RpcServiceT<'a>,
{
    type Future = MetricsFuture<S::Future>;

    fn call(&self, request: Request<'a>) -> Self::Future {
        let method = request.method_name().to_string();
        counter!(RPC_REQUEST, "method" => method.clone()).increment(1);
        MetricsFuture { fut: self.inner.call(request), method }
    }
}
/// Response future to log the response for a method call.
#[pin_project]
pub struct MetricsFuture<F> {
    #[pin]
    fut: F,
    method: String,
}

impl<F> std::fmt::Debug for MetricsFuture<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MetricsFuture")
    }
}

impl<F: Future<Output = MethodResponse>> Future for MetricsFuture<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let method = self.as_ref().method.clone();

        let fut = self.project().fut;

        let res = fut.poll(cx);
        if let Poll::Ready(rp) = &res {
            let is_subscription = rp.is_subscription() as i32;
            match rp.success_or_error {
                MethodResponseResult::Success => counter!(
                    RPC_REQUEST_SUCCESS,
                    "method" => method,
                    "is_subscription" => is_subscription.to_string()
                )
                .increment(1),
                MethodResponseResult::Failed(code) => counter!(
                    RPC_REQUEST_FAILED,
                    "method" => method,
                    "code" => code.to_string(),
                    "is_subscription" => is_subscription.to_string()
                )
                .increment(1),
            }
        }
        res
    }
}

pub fn describe_json_rpc_metrics() {
    describe_counter!(RPC_REQUEST, "The number of json rpc requests so far");
    describe_counter!(RPC_REQUEST_SUCCESS, "The number of successful json rpc requests so far");
    describe_counter!(RPC_REQUEST_FAILED, "The number of failed json rpc requests so far");
    counter!(RPC_REQUEST).absolute(0);
    counter!(RPC_REQUEST_SUCCESS).absolute(0);
    counter!(RPC_REQUEST_FAILED).absolute(0);
}

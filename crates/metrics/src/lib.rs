use crate::{
    grpc::describe_grpc_metrics, mempool::describe_mempool_metrics, rpc::describe_json_rpc_metrics,
};
use label::LabelValue;
use metrics_exporter_prometheus::PrometheusBuilder;
use metrics_util::MetricKindMask;
use std::{net::SocketAddr, time::Duration};
use tracing::info;

pub mod ethers;
pub mod grpc;
pub mod label;
pub mod mempool;
pub mod rpc;

pub fn launch_metrics_exporter(listen_addr: SocketAddr, label_value_opt: Option<Vec<LabelValue>>) {
    let mut builder = PrometheusBuilder::new();
    info!("launching Prometheus metrics exporter on {}", listen_addr);
    if let Some(label_values) = label_value_opt {
        for LabelValue { label, value } in label_values.iter() {
            builder = builder.add_global_label(label, value);
        }
    }
    builder
        .with_http_listener(listen_addr)
        .idle_timeout(
            MetricKindMask::COUNTER | MetricKindMask::HISTOGRAM,
            Some(Duration::from_secs(10)),
        )
        .install()
        .expect("failed to install Prometheus recorder");

    describe_json_rpc_metrics();
    describe_mempool_metrics();
    describe_grpc_metrics();
}

use nxzr_proto::{LogStreamRequest, LogStreamResponse};
use std::pin::Pin;
use tokio::sync::broadcast;
use tokio_stream::{wrappers::BroadcastStream, Stream, StreamExt};
use tonic::{async_trait, Request, Response, Status};

type TracingResult<T> = Result<Response<T>, Status>;
type ResponseStream<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send>>;

#[derive(Debug)]
pub struct TracingService {
    tracing_json_tx: broadcast::Sender<String>,
}

impl TracingService {
    pub fn new(tracing_json_tx: broadcast::Sender<String>) -> Self {
        Self { tracing_json_tx }
    }
}

#[async_trait]
impl nxzr_proto::tracing_server::Tracing for TracingService {
    type LogStreamStream = ResponseStream<LogStreamResponse>;
    async fn log_stream(
        &self,
        req: Request<LogStreamRequest>,
    ) -> TracingResult<Self::LogStreamStream> {
        let out_stream = BroadcastStream::new(self.tracing_json_tx.subscribe())
            .filter_map(|v| v.ok())
            .map(|val| Ok::<_, Status>(LogStreamResponse { tracing_json: val }));
        Ok(Response::new(Box::pin(out_stream) as Self::LogStreamStream))
    }
}

// #[derive(Debug)]
// pub struct NxzrService {
//     is_connected: bool,
// }

// impl nxzr_proto::nxzr_server::Nxzr for NxzrService {
//     async fn
//     fn
// }

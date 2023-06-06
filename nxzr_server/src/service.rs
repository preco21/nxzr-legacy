use nxzr_proto::{
    nxzr_server::Nxzr, ButtonControlStreamRequest, ButtonControlStreamResponse,
    ConnectSwitchRequest, ConnectSwitchResponse, GetDeviceStatusRequest, GetDeviceStatusResponse,
    GetProtocolStateRequest, GetProtocolStateResponse, ImuControlStreamRequest,
    ImuControlStreamResponse, ReconnectSwitchRequest, ReconnectSwitchResponse,
    StickControlStreamRequest, StickControlStreamResponse,
};
use std::pin::Pin;
use tokio_stream::Stream;
use tonic::{async_trait, Request, Response, Status, Streaming};

type ServiceResult<T> = Result<Response<T>, Status>;
type ResponseStream<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send>>;

#[derive(Debug)]
pub struct NxzrService {}

impl NxzrService {}

#[async_trait]
impl Nxzr for NxzrService {
    async fn get_device_status(
        &self,
        req: Request<GetDeviceStatusRequest>,
    ) -> ServiceResult<GetDeviceStatusResponse> {
        unimplemented!()
    }

    type ConnectSwitchStream = ResponseStream<ConnectSwitchResponse>;
    async fn connect_switch(
        &self,
        req: Request<ConnectSwitchRequest>,
    ) -> ServiceResult<Self::ConnectSwitchStream> {
        unimplemented!()
    }

    type ReconnectSwitchStream = ResponseStream<ReconnectSwitchResponse>;
    async fn reconnect_switch(
        &self,
        req: Request<ReconnectSwitchRequest>,
    ) -> ServiceResult<Self::ReconnectSwitchStream> {
        unimplemented!()
    }

    async fn get_protocol_state(
        &self,
        req: Request<GetProtocolStateRequest>,
    ) -> ServiceResult<GetProtocolStateResponse> {
        unimplemented!()
    }

    type ButtonControlStreamStream = ResponseStream<ButtonControlStreamResponse>;
    async fn button_control_stream(
        &self,
        req: Request<Streaming<ButtonControlStreamRequest>>,
    ) -> ServiceResult<Self::ButtonControlStreamStream> {
        unimplemented!()
    }

    type StickControlStreamStream = ResponseStream<StickControlStreamResponse>;
    async fn stick_control_stream(
        &self,
        req: Request<Streaming<StickControlStreamRequest>>,
    ) -> ServiceResult<Self::StickControlStreamStream> {
        unimplemented!()
    }

    type ImuControlStreamStream = ResponseStream<ImuControlStreamResponse>;
    async fn imu_control_stream(
        &self,
        req: Request<Streaming<ImuControlStreamRequest>>,
    ) -> ServiceResult<Self::ImuControlStreamStream> {
        unimplemented!()
    }

    // type ConnectSwitchStream = ResponseStream<ConnectSwitchResponse>;
    // type LogStreamStream = ResponseStream<LogStreamResponse>;
    // async fn log_stream(
    //     &self,
    //     req: Request<LogStreamRequest>,
    // ) -> ServiceResult<Self::LogStreamStream> {
    //     let out_stream = BroadcastStream::new(self.tracing_json_tx.subscribe())
    //         .filter_map(|v| v.ok())
    //         .map(|val| Ok::<_, Status>(LogStreamResponse { tracing_json: val }));
    //     Ok(Response::new(Box::pin(out_stream) as Self::LogStreamStream))
    // }
}

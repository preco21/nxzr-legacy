use crate::server::Server;
use nxzr_device::device;
use nxzr_proto::{
    nxzr_server::Nxzr, ButtonControlStreamRequest, ButtonControlStreamResponse,
    ConnectSwitchRequest, ConnectSwitchResponse, GetDeviceStatusRequest, GetDeviceStatusResponse,
    GetProtocolStateRequest, GetProtocolStateResponse, ImuControlStreamRequest,
    ImuControlStreamResponse, ReconnectSwitchRequest, ReconnectSwitchResponse,
    StickControlStreamRequest, StickControlStreamResponse,
};
use std::{pin::Pin, sync::Arc};
use tokio::sync::{mpsc, Mutex};
use tokio_stream::Stream;
use tonic::{async_trait, Request, Response, Status, Streaming};

type ServiceResult<T> = Result<Response<T>, Status>;
type ResponseStream<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send>>;

#[derive(Debug)]
pub struct NxzrService {
    device: device::Device,
    server: Mutex<Option<Server>>,
}

impl NxzrService {
    pub async fn new() -> anyhow::Result<Self> {
        Ok(Self {
            // Note that the device will only rely on the first adapter (e.g.
            // hci0), and will never restart due to incompatibilities with the
            // bluez `input` plugin.
            //
            // This is guaranteed to not happen because we will only serve the
            // daemon in managed container of the WSL.
            device: device::Device::new(device::DeviceConfig::default()).await?,
            server: Mutex::new(None),
        })
    }
}

#[async_trait]
impl Nxzr for NxzrService {
    async fn get_device_status(
        &self,
        req: Request<GetDeviceStatusRequest>,
    ) -> ServiceResult<GetDeviceStatusResponse> {
        let protocol = {
            let guard = self.server.lock().await;
            let Some(server) = &*guard else {
                return Err(tonic::Status::aborted("foo"));
            };
            server.protocol()
        };
        unimplemented!()
    }

    type ConnectSwitchStream = ResponseStream<ConnectSwitchResponse>;
    async fn connect_switch(
        &self,
        req: Request<ConnectSwitchRequest>,
    ) -> ServiceResult<Self::ConnectSwitchStream> {
        self.server.lock().await;
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

pub struct NxzrServiceHandle {
    _close_rx: mpsc::Receiver<()>,
}

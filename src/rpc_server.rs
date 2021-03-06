use tokio::sync::mpsc::UnboundedSender;
use crate::network::CommomNetMsg;
use crate::rpc_client::RpcClientMsg;
use anyhow::Result;
use cita_cloud_proto::common::Empty;
use cita_cloud_proto::common::SimpleResponse;
use prost::Message;
use cita_cloud_proto::network::{
    network_service_server::NetworkService, network_service_server::NetworkServiceServer,
    NetworkMsg, NetworkStatusResponse, RegisterInfo,
};
use log::{debug, warn};
use tonic::{transport::Server, Request, Response, Status};

pub struct RpcServer {
    net_event_sender: UnboundedSender<CommomNetMsg>,
    to_rpc_cli_tx: UnboundedSender<RpcClientMsg>,
}

impl RpcServer {
    fn new(
        net_event_sender: UnboundedSender<CommomNetMsg>,
        to_rpc_cli_tx: UnboundedSender<RpcClientMsg>,
    ) -> Self {
        Self {
            net_event_sender,
            to_rpc_cli_tx,
        }
    }

    pub async fn run(
        net_event_sender: UnboundedSender<CommomNetMsg>,
        to_rpc_cli_tx: UnboundedSender<RpcClientMsg>,
        serv_port: String,
    ) -> Result<()> {
        let addr_str = format!("127.0.0.1:{}", serv_port);
        let addr = addr_str.parse()?;

        let rpc_serv = RpcServer::new(net_event_sender, to_rpc_cli_tx);

        Server::builder()
            .add_service(NetworkServiceServer::new(rpc_serv))
            .serve(addr)
            .await?;
        Ok(())
    }
}

#[tonic::async_trait]
impl NetworkService for RpcServer {
    async fn send_msg(
        &self,
        request: Request<NetworkMsg>,
    ) -> Result<Response<SimpleResponse>, Status> {
        debug!("send_msg request: {:?}", request);

        let msg = request.into_inner();
        let mut buf: Vec<u8> = Vec::new();
        if msg.encode(&mut buf).is_ok() {
            let event = CommomNetMsg::SendMessage(msg.origin as usize, buf);
            if let Err(e) = self.net_event_sender.send(event) {
                warn!("RpcServer send failed: `{}`", e);
            }
            let reply = SimpleResponse { is_success: true };
            Ok(Response::new(reply))
        } else {
            Err(Status::internal("encode msg failed"))
        }
    }

    async fn broadcast(
        &self,
        request: Request<NetworkMsg>,
    ) -> Result<Response<SimpleResponse>, Status> {
        debug!("broadcast request: {:?}", request);

        let msg = request.into_inner();
        let mut buf: Vec<u8> = Vec::new();
        if msg.encode(&mut buf).is_ok() {
            let event = CommomNetMsg::SendMessage(0, buf);
            if let Err(e) = self.net_event_sender.send(event) {
                warn!("RpcServer broadcast failed: `{}`", e);
            }
            let reply = SimpleResponse { is_success: true };
            Ok(Response::new(reply))
        } else {
            Err(Status::internal("encode msg failed"))
        }
    }

    async fn get_network_status(
        &self,
        request: Request<Empty>,
    ) -> Result<Response<NetworkStatusResponse>, Status> {
        debug!("register_endpoint request: {:?}", request);
        use tokio::sync::oneshot;
        let (tx, rx) = oneshot::channel();
        let event = CommomNetMsg::GetPeerCount(tx);
        if let Err(e) = self.net_event_sender.send(event) {
            warn!("RpcServer get_network_status failed: `{}`", e);
        }

        let peer_count = rx.await.unwrap_or(0) as u64;
        let reply = NetworkStatusResponse { peer_count };
        Ok(Response::new(reply))
    }

    async fn register_network_msg_handler(
        &self,
        request: Request<RegisterInfo>,
    ) -> Result<Response<SimpleResponse>, Status> {
        debug!("register_network_msg_handler request: {:?}", request);

        let info = request.into_inner();
        let module_name = info.module_name;
        let hostname = info.hostname;
        let port = info.port;

        let _ = self.to_rpc_cli_tx
            .send(RpcClientMsg::ModPort(module_name, hostname, port));

        let reply = SimpleResponse { is_success: true };
        Ok(Response::new(reply))
    }
}

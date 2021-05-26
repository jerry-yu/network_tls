use tokio::sync::mpsc;
use tokio::sync::mpsc::unbounded_channel;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time::interval;

use cita_cloud_proto::common::Empty;
use cita_cloud_proto::common::SimpleResponse;
use cita_cloud_proto::network::network_msg_handler_service_client::NetworkMsgHandlerServiceClient;
use cita_cloud_proto::network::{
    network_service_server::NetworkService, network_service_server::NetworkServiceServer,
    NetworkMsg, NetworkStatusResponse, RegisterInfo};
use std::collections::HashMap;

pub enum RpcClientMsg {
    ModPort(String,String,String),
    Msg(NetworkMsg),
}

pub(crate) struct RpcClient {
    rpc_rx :  UnboundedReceiver<RpcClientMsg>,
    mod_senders: HashMap<String, NetworkMsgHandlerServiceClient<tonic::transport::Channel>>,
    mod_port :  HashMap<String,String>,
}


impl RpcClient {
    pub fn new(rpc_rx:UnboundedReceiver<RpcClientMsg>) -> RpcClient {
        RpcClient {
            rpc_rx,
            mod_port : HashMap::new(),
            mod_senders: HashMap::new(),

        }
    }

    pub async fn run(rpc_rx:UnboundedReceiver<RpcClientMsg>) {
        let mut rpc = RpcClient::new(rpc_rx);

        while let Some(msg) = rpc.rpc_rx.recv().await {

        }
    }

}


// async fn dispatch_network_msg(
//     &self,
//     port: String,
//     msg: NetworkMsg,
// ) -> Result<(), Box<dyn std::error::Error>> {
//     let mut client = {
//         let map = client_map.read().await;
//         map.get(&port).cloned()
//     };

//     if client.is_none() {
//         let dest_addr = format!("http://127.0.0.1:{}", port);
//         let c = NetworkMsgHandlerServiceClient::connect(dest_addr).await?;
//         client_map.write().await.insert(port, c.clone());
//         client.replace(c);
//     }

//     let mut client = client.unwrap();
//     let request = Request::new(msg);
//     let _response = client.process_network_msg(request).await?;

//     Ok(())
// }
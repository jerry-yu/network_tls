use log::{info,warn};
use tokio::sync::mpsc;
use tokio::sync::mpsc::unbounded_channel;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time::interval;

use crate::network::SessionID;
use cita_cloud_proto::common::Empty;
use cita_cloud_proto::common::SimpleResponse;
use cita_cloud_proto::network::network_msg_handler_service_client::NetworkMsgHandlerServiceClient;
use cita_cloud_proto::network::{
    network_service_server::NetworkService, network_service_server::NetworkServiceServer,
    NetworkMsg, NetworkStatusResponse, RegisterInfo,
};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use tonic::{transport::Server, Request, Response, Status};
use prost::Message;

#[derive(Debug)]
pub enum RpcClientMsg {
    ModPort(String, String, String),
    FromNetMsg(SessionID, Vec<u8>),
}

pub(crate) struct RpcClient {
    rpc_rx: UnboundedReceiver<RpcClientMsg>,
    mod_senders: HashMap<String, NetworkMsgHandlerServiceClient<tonic::transport::Channel>>,
    mod_port: HashMap<String, String>,
}

impl RpcClient {
    pub fn new(rpc_rx: UnboundedReceiver<RpcClientMsg>) -> RpcClient {
        RpcClient {
            rpc_rx,
            mod_port: HashMap::new(),
            mod_senders: HashMap::new(),
        }
    }

    pub async fn run(rpc_rx: UnboundedReceiver<RpcClientMsg>) {
        let mut rpc = RpcClient::new(rpc_rx);
        let mut mod_clients = HashMap::new();
        let mut mod_infos = HashMap::new();

        while let Some(msg) = rpc.rpc_rx.recv().await {
            match msg {
                RpcClientMsg::ModPort(mod_name, hostname, port) => {
                    let dest_addr = format!("http://{}:{}", hostname, port);
                    mod_infos.insert(mod_name.clone(), dest_addr.clone());
                    if let Ok(client) =
                        NetworkMsgHandlerServiceClient::connect(dest_addr.clone()).await
                    {
                        mod_clients.insert(mod_name, client);
                    } else {
                        warn!("connect rpc not ok mod_name {},url {}", mod_name, dest_addr);
                    }
                }
                RpcClientMsg::FromNetMsg(sid, data) => {
                    let msg = NetworkMsg::decode(data.as_slice());
                    if msg.is_err() {
                        continue;
                    }
                    let msg = msg.unwrap();
                    let mod_name = msg.module.clone();
                    let request = Request::new(msg);
                    match mod_clients.entry(mod_name.clone()) {
                        Entry::Occupied(mut o) => {
                            let _response = o.get_mut().process_network_msg(request).await;
                            info!("process_network_msg {:?}",_response);
                        }
                        Entry::Vacant(v) => {
                            if let Some(dest_addr) = mod_infos.get(&mod_name) {
                                info!("reconnect {} {}",mod_name,dest_addr);
                                if let Ok(mut c) =
                                    NetworkMsgHandlerServiceClient::connect(dest_addr.to_owned())
                                        .await
                                {
                                    let _response = c.process_network_msg(request).await;
                                    v.insert(c);
                                }
                            }
                        }
                    }
                }
            }
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

// Copyright Rivtower Technologies LLC.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

mod config;
mod rpc_client;
mod rpc_server;
mod network;

use clap::Clap;
use git_version::git_version;
use log::{debug, info, warn};
use std::net::ToSocketAddrs;

const GIT_VERSION: &str = git_version!(
    args = ["--tags", "--always", "--dirty=-modified"],
    fallback = "unknown"
);
const GIT_HOMEPAGE: &str = "https://github.com/cita-cloud/network_direct";

/// network service
#[derive(Clap)]
#[clap(version = "0.1.0", author = "Rivtower Technologies.")]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    /// print information from git
    #[clap(name = "git")]
    GitInfo,
    /// run this service
    #[clap(name = "run")]
    Run(RunOpts),
}

/// A subcommand for run
#[derive(Clap)]
struct RunOpts {
    /// Sets grpc port of this service.
    #[clap(short = 'p', long = "port", default_value = "50000")]
    grpc_port: String,
    /// Sets path of network key file.
    /// It's not used for network_direct. Leave it here for compatibility.
    #[allow(unused)]
    #[clap(short = 'k', long = "key_file", default_value = "network-key")]
    key_file: String,
}

fn main() {
    ::std::env::set_var("RUST_BACKTRACE", "full");

    let opts: Opts = Opts::parse();

    match opts.subcmd {
        SubCommand::GitInfo => {
            println!("git version: {}", GIT_VERSION);
            println!("homepage: {}", GIT_HOMEPAGE);
        }
        SubCommand::Run(opts) => {
            // init log4rs
            log4rs::init_file("network-log4rs.yaml", Default::default()).unwrap();
            info!("grpc port of this service: {}", opts.grpc_port);
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(run(opts));
        }
    }
}

use config::NetConfig;
use prost::Message;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::mpsc::unbounded_channel;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time::interval;

use cita_cloud_proto::common::Empty;
use cita_cloud_proto::common::SimpleResponse;
use cita_cloud_proto::network::network_msg_handler_service_client::NetworkMsgHandlerServiceClient;
use cita_cloud_proto::network::{
    network_service_server::NetworkService, network_service_server::NetworkServiceServer,
    NetworkMsg, NetworkStatusResponse, RegisterInfo,
};
use tonic::{transport::Server, Request, Response, Status};


async fn run(opts: RunOpts) {
    let path = "network-config.toml";
    let buffer = std::fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("Error while loading config: [{}]", err));
    let config = NetConfig::new(&buffer);
    let listen_addr :String  = format!("0.0.0.0:{}", config.port);
    let peers: Vec<String> = config
        .peers
        .into_iter()
        .map(|peer| format!("{}:{}", peer.ip, peer.port))
        .collect();

    let (to_net_tx, to_net_rx) = unbounded_channel();
    let (from_net_tx, from_net_rx) = unbounded_channel();

    let mut net_op = network::CommonNetwork::new(listen_addr, to_net_tx.clone(),from_net_rx);
   
    let (to_rpc_cli_tx,to_rpc_cli_rx) = unbounded_channel();

    tokio::spawn(rpc_client::RpcClient::run(to_rpc_cli_rx));

    //let (to_rpc_serv_tx,to_rpc_serv_rx) = unbounded_channel();

    tokio::spawn( rpc_server::RpcServer::run(to_net_tx, to_rpc_cli_tx,opts.grpc_port) );
   
    net_op.run().await;
}

// async fn keep_connection(peers: Vec<String>, net_event_sender: mpsc::Sender<NetEvent>) {
//     let mut recheck_interval = interval(Duration::from_secs(15));
//     loop {
//         recheck_interval.tick().await;
//         for addr in peers.iter() {
//             let event = NetEvent::OutboundConnection { addr: addr.clone() };
//             net_event_sender.send(event).await.unwrap();
//         }
//     }
// }


// async fn dispatch_network_msg(
//     client_map: Arc<
//         RwLock<HashMap<String, NetworkMsgHandlerServiceClient<tonic::transport::Channel>>>,
//     >,
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

// async fn run_network(
//     mut network_rx: UnboundedReceiver<(u64, Vec<u8>)>,
//     dispatch_table: Arc<RwLock<HashMap<String, (String, String)>>>,
// ) {
//     let client_map = Arc::new(RwLock::new(HashMap::<
//         String,
//         NetworkMsgHandlerServiceClient<tonic::transport::Channel>,
//     >::new()));
//     loop {
//         if let Some(msg) = network_rx.recv().await {
//             let (sid, payload) = msg;
//             debug!("received msg {:?} from {}", payload, sid);

//             match NetworkMsg::decode(payload.as_slice()) {
//                 Ok(mut msg) => {
//                     msg.origin = sid as u64;
//                     let dispatch_table = dispatch_table.clone();
//                     let client_map = client_map.clone();
//                     tokio::spawn(async move {
//                         let port = {
//                             dispatch_table.get(&msg.module).cloned().map(|(_, port)| port)
//                         };
//                         if let Some(port) = port {
//                             if let Err(e) = dispatch_network_msg(client_map, port, msg).await {
//                                 warn!("dispatch error: {:?}", e);
//                             }
//                         } else {
//                             warn!("dipatch port not found");
//                         }
//                     });
//                 }
//                 Err(e) => {
//                     warn!("network msg decode failed: {}", e);
//                 }
//             }
//         }
//     }
// }

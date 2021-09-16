use crate::controller::ControlInfoMsg;
use crate::rpc_client::RpcClientMsg;
use anyhow::Result;
use log::{info, trace, warn};
use std::collections::BTreeMap;
use std::net::SocketAddr;

use tokio::sync::oneshot;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;

use futures::StreamExt;
use libp2p::{
    core::upgrade,
    identity,
    mplex,
    noise,
    swarm::{NetworkBehaviourEventProcess, SwarmBuilder, SwarmEvent},
    // `TokioTcpConfig` is available through the `tcp-tokio` feature.
    tcp::TokioTcpConfig,
    Multiaddr,
    NetworkBehaviour,
    PeerId,
    Transport,
    Swarm
};
use transfer::{TransferConfig,Transfer};


pub type SessionID = usize;

#[derive(Debug)]
pub enum CommomNetMsg {
    SendMessage(SessionID, Vec<u8>),
    ToConnect(String),
    ToDisConnect(SessionID),
    GetPeerCount(oneshot::Sender<usize>),
}

#[derive(Debug)]
pub struct CommonNetwork {
    listen_addr: String,
    net_sender: UnboundedSender<RpcClientMsg>,
    net_receiver: UnboundedReceiver<CommomNetMsg>,
    ctl_sender: UnboundedSender<ControlInfoMsg>,
}

// async fn read_net_and_send(
//     session_id: usize,
//     mut reader: OwnedReadHalf,
//     net_sender: UnboundedSender<RpcClientMsg>,
//     ctl_sender: UnboundedSender<ControlInfoMsg>,
//     mut term_rx: Receiver<()>,
// ) -> Result<()> {
//     loop {
//         tokio::select! {
//             len = reader.read_u64() => {
//                 let len = len.unwrap_or(0);
//                 if len == 0 {
//                     break;
//                 }
//                 let mut buffer = Vec::with_capacity(len as usize);
//                 let len = reader.read_exact(&mut buffer[..]).await.unwrap_or(0);
//                 if len == 0 {
//                     break;
//                 }
//                 net_sender
//                     .send(RpcClientMsg::FromNetMsg(session_id, buffer))
//                     .unwrap();
//             }
//             _= term_rx.recv() => {
//                 break;
//             }
//         }
//     }
//     let msg = ControlInfoMsg::DisConnected(session_id);
//     ctl_sender.send(msg)?;
//     Ok(())
// }

impl CommonNetwork {
    pub fn new(
        listen_addr: String,
        net_sender: UnboundedSender<RpcClientMsg>,
        net_receiver: UnboundedReceiver<CommomNetMsg>,
        ctl_sender: UnboundedSender<ControlInfoMsg>,
    ) -> CommonNetwork {
        CommonNetwork {
            listen_addr,
            net_sender,
            net_receiver,
            ctl_sender,
        }
    }

    // pub async fn run(&mut self) -> Result<()> {
    //     let addr: SocketAddr = self.listen_addr.parse().unwrap();
    //     let mut listener = TcpListener::bind(addr).await?;
    //     let mut session_id: SessionID = 1;
    //     let mut cons = BTreeMap::new();

    //     loop {
    //         tokio::select! {
    //             con = listener.accept() => {
    //                 if let Ok((mut con,_)) = con {
    //                     let (term_tx,term_rx) = mpsc::channel(1);
    //                     let (reader,writer) = con.into_split();
    //                     info!("accept get session id {}",session_id);
    //                     cons.insert(session_id, (writer,term_tx));
    //                     tokio::spawn(read_net_and_send(session_id,reader,self.net_sender.clone(),self.ctl_sender.clone(),term_rx));
    //                     let cmsg = ControlInfoMsg::Connected(session_id,String::new());
    //                     self.ctl_sender.send(cmsg);
    //                     session_id += 1;
    //                 }
    //             },
    //             msg = self.net_receiver.recv() => {
    //                 if let Some(msg) = msg {
    //                     match msg {
    //                         CommomNetMsg::SendMessage(sid,data) => {
    //                             let mut err_sid = Vec::new();
    //                             if sid == 0 {
    //                                 for (id,(writer,tx)) in cons.iter_mut() {
    //                                     if let Err(_) =  writer.write_all(&data).await {
    //                                         tx.send(()).await;
    //                                         err_sid.push(*id);
    //                                    }
    //                                 }
    //                             } else if let Some((writer,tx)) = cons.get_mut(&sid) {
    //                                 if let Err(_) =  writer.write_all(&data).await {
    //                                     tx.send(()).await;
    //                                     err_sid.push(sid);
    //                                }
    //                             } else {
    //                                 info!("send message failed, id {} not found",sid);
    //                             }

    //                             for i in err_sid {
    //                                 cons.remove(&i);
    //                             }
    //                         },
    //                         CommomNetMsg::ToConnect(url) => {
    //                             info!("  ToConnect {}  ",url);
    //                             if let Ok(con) = TcpStream::connect(url.clone()).await {
    //                                 let (term_tx,term_rx) = mpsc::channel(1);
    //                                 let (reader,writer) = con.into_split();
    //                                 cons.insert(session_id, (writer,term_tx));
    //                                 tokio::spawn(read_net_and_send(session_id,reader,self.net_sender.clone(),self.ctl_sender.clone(),term_rx));
    //                                 let cmsg = ControlInfoMsg::Connected(session_id,url);
    //                                 self.ctl_sender.send(cmsg);
    //                                 session_id +=1;
                                   
    //                             } else {
    //                                 info!(" connect {} not ok ",url);
    //                             }
    //                         }
    //                         CommomNetMsg::ToDisConnect(sid) => {
    //                             info!(" ToDisConnect id {}",sid);
    //                             if let Some((_,sender)) = cons.remove(&sid) {
    //                                 sender.send(()).await;
                                   
    //                             }
    //                         }
    //                         CommomNetMsg::GetPeerCount(tx) => {
    //                             let peer_count = cons.len();
    //                             info!(" GetPeerCount {}",peer_count);
    //                             tx.send(peer_count as u64);
    //                         }
    //                     }
    //                 }
    //             }

    //         }
    //     }
    //     Ok(())
    // }

    pub async fn run2(&mut self) -> Result<()> {
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        println!("Local peer id: {:?}", local_peer_id);
   
        let transport = libp2p::development_transport(local_key).await?;
        let mut swarm = {
            let mut behaviour = Transfer::new(TransferConfig::default());
            SwarmBuilder::new(transport, behaviour, local_peer_id)
                // We want the connection background tasks to be spawned
                // onto the tokio runtime.
                .executor(Box::new(|fut| {
                    tokio::spawn(fut);
                }))
                .build()
        };

        swarm.listen_on(self.listen_addr.parse()?)?;

        loop {
            tokio::select! {
                msg = self.net_receiver.recv() => {
                    if let Some(msg) = msg {
                        match msg {
                            CommomNetMsg::SendMessage(sid,data) => {
                                if sid == 0 {
                                    swarm.behaviour_mut().broadcast(data.into());
                                    info!("broadcast ");
                                } else {
                                    swarm.behaviour_mut().send_to_peer(sid,data.into());
                                    info!("send message id {} ",sid);
                                }
                            },
                            CommomNetMsg::ToConnect(remote) => {
                                swarm.dial_addr(remote.parse()?)?;
                                println!("Dialed {}", remote);
                            }
                            CommomNetMsg::ToDisConnect(sid) => {
                                info!(" ToDisConnect id {}",sid);
                                // TOBE continue
                            }
                            CommomNetMsg::GetPeerCount(tx) => {
                                // let peer_count = cons.len();
                                // info!(" GetPeerCount {}",peer_count);
                                // tx.send(peer_count as u64);
                            }
                        }
                    }
                },

                // event = swarm.select_next_some() => {
                //     match event {
                //         SwarmEvent::NewListenAddr { address, .. } => println!("Listening on {:?}", address),
                //         SwarmEvent::Behaviour(event) => println!("{:?}", event),
                //         x => println!("swarn select next get {:?}", x)
                //     }
                // },
            }
         }

        Ok(())
    }


}

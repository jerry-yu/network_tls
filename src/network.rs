use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::OwnedReadHalf;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::tcp::WriteHalf;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::interval;

use crate::controller::ControlMsg;
use crate::rpc_client::RpcClientMsg;
use anyhow::Result;
use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio_stream::wrappers::TcpListenerStream;
use tokio_stream::StreamExt;

pub type SessionID = u64;

#[derive(Debug)]
pub enum CommomNetMsg {
    SendMessage(SessionID, Vec<u8>),
    GetPeerCount(oneshot::Sender<u64>),
}

#[derive(Debug)]
pub struct CommonNetwork {
    listen_addr: String,
    net_sender: UnboundedSender<RpcClientMsg>,
    net_receiver: UnboundedReceiver<CommomNetMsg>,
    cons: BTreeMap<SessionID, OwnedWriteHalf>,
    ctl_sender: UnboundedSender<ControlMsg>,
}

async fn read_net_and_send(
    session_id: u64,
    mut reader: OwnedReadHalf,
    net_sender: UnboundedSender<RpcClientMsg>,
    ctl_sender: UnboundedSender<ControlMsg>,
) -> Result<()> {
    loop {
        let len = reader.read_u64().await.unwrap_or(0);
        if len == 0 {
            break;
        }
        let mut buffer = Vec::with_capacity(len as usize);
        let len = reader.read_exact(&mut buffer[..]).await.unwrap_or(0);
        if len == 0 {
            break;
        }
        net_sender
            .send(RpcClientMsg::FromNetMsg(session_id, buffer))
            .unwrap();
    }
    let msg = ControlMsg::DisConnected(session_id);
    ctl_sender.send(msg)?;
    Ok(())
}

impl CommonNetwork {
    pub fn new(
        listen_addr: String,
        net_sender: UnboundedSender<RpcClientMsg>,
        net_receiver: UnboundedReceiver<CommomNetMsg>,
        ctl_sender: UnboundedSender<ControlMsg>,
    ) -> CommonNetwork {
        CommonNetwork {
            listen_addr,
            net_sender,
            net_receiver,
            cons: BTreeMap::new(),
            ctl_sender,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let addr: SocketAddr = self.listen_addr.parse().unwrap();
        let mut listener = TcpListener::bind(addr).await?;
        let mut session_id: SessionID = 1;

        tokio::select! {
            con = listener.accept() => {
                if let Ok((mut con,_)) = con {
                    let (reader,writer) = con.into_split();
                    self.cons.insert(session_id, writer);
                    session_id +=1;

                    tokio::spawn(read_net_and_send(session_id,reader,self.net_sender.clone(),self.ctl_sender.clone()));

                }
            },
            msg = self.net_receiver.recv() => {

                //tokio::spawn();
            }

        }

        Ok(())
    }
}

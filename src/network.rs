use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::OwnedReadHalf;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::interval;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::net::tcp::WriteHalf;

use std::net::SocketAddr;
use std::collections::BTreeMap;
use tokio_stream::wrappers::TcpListenerStream;
use tokio_stream::StreamExt;
use anyhow::Result;
use tokio::sync::oneshot;

pub type SessionID = u64;

#[derive(Debug)]
pub enum CommomNetMsg {
    SendMessage(SessionID,Vec<u8>),
    ConnInfo(SessionID,String),
    NetRecvMesage(SessionID,Vec<u8>),
    ToConnect(String),
    GetPeerCount(oneshot::Sender::<u64>)
} 

#[derive(Debug)]
pub struct CommonNetwork {
    listen_addr : String,
    net_sender: UnboundedSender<CommomNetMsg>,
    net_receiver: UnboundedReceiver<CommomNetMsg>,
    cons : BTreeMap<SessionID,OwnedWriteHalf>,
}

pub (crate) async fn read_net_and_send(session_id:u64,
    mut reader:OwnedReadHalf,
    net_sender: UnboundedSender<CommomNetMsg>) -> Result<()> {
    let len = reader.read_u64().await?;
    let mut buffer = Vec::new();
    let _ = reader.read_exact(&mut buffer[..]).await?;

    net_sender.send(CommomNetMsg::NetRecvMesage(session_id,buffer))?;

    Ok(())
}

impl CommonNetwork {
    pub fn new(listen_addr : String, net_sender: UnboundedSender<CommomNetMsg>,net_receiver: UnboundedReceiver<CommomNetMsg>) -> CommonNetwork {
        CommonNetwork {
            listen_addr,
            net_sender,
            net_receiver,
            cons : BTreeMap::new(),
        }
    }


    pub async fn run(&mut self) -> Result<()> {
        let addr:SocketAddr = self.listen_addr.parse().unwrap();
        let mut listener = TcpListener::bind(addr).await?;
        let mut session_id :SessionID = 1;

        tokio::select! {
            con = listener.accept() => {
                if let Ok((mut con,_)) = con {
                    let (reader,writer) = con.into_split();
                    self.cons.insert(session_id, writer);
                    session_id +=1;

                    tokio::spawn(read_net_and_send(session_id,reader,self.net_sender.clone()));

                }
            },
            msg = self.net_receiver.recv() => {

                //tokio::spawn();
            }

        }

        Ok(())

    }
}


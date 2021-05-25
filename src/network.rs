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
#[derive(Debug)]
pub enum CommomNetMsg {
    SendMessage(u64,Vec<u8>),
    ConnInfo(u64,String),
    GetMessage(u64,Vec<u8>),
} 

#[derive(Debug)]
pub struct CommonNetwork {
    listen_addr : String,
    net_receiver: UnboundedReceiver<CommomNetMsg>,
    net_sender: UnboundedSender<CommomNetMsg>,
    session_id: u64,
    cons : BTreeMap<u64,OwnedWriteHalf>,
}

pub (crate) async fn read_net_and_send(session_id:u64,
    mut reader:OwnedReadHalf,
    net_sender: UnboundedSender<CommomNetMsg>) -> Result<()> {
    let len = reader.read_u64().await?;
    let mut buffer = Vec::new();
    let _ = reader.read_exact(&mut buffer[..]).await?;

    net_sender.send(CommomNetMsg::GetMessage(session_id,buffer))?;

    Ok(())
}

impl CommonNetwork {
    pub async fn run(&mut self,addr : SocketAddr) -> Result<()> {
        let mut listener = TcpListener::bind(addr).await?;

        tokio::select! {
            con = listener.accept() => {
                if let Ok((mut con,_)) = con {
                    let (reader,writer) = con.into_split();
                    self.cons.insert(self.session_id, writer);
                    self.session_id +=1;

                    tokio::spawn(read_net_and_send(self.session_id,reader,self.net_sender.clone()));

                }
                
            },
            msg = self.net_receiver.recv() => {

                //tokio::spawn();
            }

        }

        Ok(())

    }
}


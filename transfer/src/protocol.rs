use futures::future::BoxFuture;
use futures::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use libp2p::core::{upgrade, InboundUpgrade, OutboundUpgrade, UpgradeInfo};
use std::io::{Error, Result};
use std::sync::Arc;

const PROTOCOL_INFO: &[u8] = b"/transfer/1.0.0";

// #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
// pub struct Topic {
//     len: u8,
//     bytes: [u8; 64],
// }

// impl Topic {
//     pub const MAX_TOPIC_LENGTH: usize = 64;

//     pub fn new(topic: &[u8]) -> Self {
//         let mut bytes = [0u8; 64];
//         bytes[..topic.len()].copy_from_slice(topic);
//         Self {
//             len: topic.len() as _,
//             bytes,
//         }
//     }
// }

// impl std::ops::Deref for Topic {
//     type Target = [u8];

//     fn deref(&self) -> &Self::Target {
//         self.as_ref()
//     }
// }

// impl AsRef<[u8]> for Topic {
//     fn as_ref(&self) -> &[u8] {
//         &self.bytes[..(self.len as usize)]
//     }
// }

#[derive(Clone, Debug, PartialEq)]
pub enum Message {
    Send(Arc<[u8]>),
}

impl Message {
    fn from_bytes(msg: &[u8]) -> Result<Self> {
        Ok(Message::Send(msg.into()))
    }

    fn to_bytes(&self) -> Vec<u8> {
        match self {
            Message::Send(msg) => {
                let mut buf = Vec::new();
                buf.extend_from_slice(msg);
                buf
            },
        }
    }
}

#[derive(Clone, Debug)]
pub struct TransferConfig {
    max_buf_size: usize,
}

impl Default for TransferConfig {
    fn default() -> Self {
        Self {
            max_buf_size: 1024 * 1024 * 4,
        }
    }
}

impl UpgradeInfo for TransferConfig {
    type Info = &'static [u8];
    type InfoIter = std::iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        std::iter::once(PROTOCOL_INFO)
    }
}

impl<TSocket> InboundUpgrade<TSocket> for TransferConfig
where
    TSocket: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    type Output = Message;
    type Error = Error;
    type Future = BoxFuture<'static, Result<Self::Output>>;

    fn upgrade_inbound(self, mut socket: TSocket, _info: Self::Info) -> Self::Future {
        Box::pin(async move {
            let packet = upgrade::read_length_prefixed(&mut socket, self.max_buf_size).await?;
            socket.close().await?;
            let request = Message::from_bytes(&packet)?;
            Ok(request)
        })
    }
}

impl UpgradeInfo for Message {
    type Info = &'static [u8];
    type InfoIter = std::iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        std::iter::once(PROTOCOL_INFO)
    }
}

impl<TSocket> OutboundUpgrade<TSocket> for Message
where
    TSocket: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    type Output = ();
    type Error = Error;
    type Future = BoxFuture<'static, Result<Self::Output>>;

    fn upgrade_outbound(self, mut socket: TSocket, _info: Self::Info) -> Self::Future {
        Box::pin(async move {
            let bytes = self.to_bytes();
            upgrade::write_length_prefixed(&mut socket, bytes).await?;
            socket.close().await?;
            Ok(())
        })
    }
}
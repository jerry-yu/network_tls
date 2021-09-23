use protocol::Message;
use fnv::{FnvHashMap};
use libp2p::core::connection::ConnectionId;
use libp2p::swarm::{
    NetworkBehaviour, NetworkBehaviourAction, NotifyHandler, OneShotHandler, PollParameters,
};
use libp2p::{Multiaddr, PeerId,core::ConnectedPoint};
use std::collections::VecDeque;
use std::sync::Arc;
use std::task::{Context, Poll};
mod protocol;
pub use protocol::{TransferConfig};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransferEvent {
    //ToBeSend(PeerId, Arc<[u8]>),
    Received(PeerId,ConnectionId,Arc<[u8]>),
    Nothing,
}

#[derive(Debug, Default)]
pub struct Transfer {
    config: TransferConfig,
    //peers: FnvHashMap<PeerId,ConnectionId>,
    peers : FnvHashMap<usize,(PeerId,ConnectionId)>,
    events: VecDeque<NetworkBehaviourAction<Message, TransferEvent>>,
    cur_id: usize,
}

impl Transfer {
    pub fn new(config: TransferConfig) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }

    pub fn broadcast(&mut self, msg: Arc<[u8]>) {
        let msg = Message::Send(msg);
        for (peer,_) in self.peers.values() {
            self.events
                .push_back(NetworkBehaviourAction::NotifyHandler {
                    peer_id: *peer,
                    event: msg.clone(),
                    handler: NotifyHandler::Any,
                });
        }
    }

    pub fn send_to_peer(&mut self,session_id : usize, msg: Arc<[u8]>) {
        let msg = Message::Send(msg);
        if let Some((peer,con)) = self.peers.get(&session_id) {
            self.events
            .push_back(NetworkBehaviourAction::NotifyHandler {
                peer_id: *peer,
                event: msg.clone(),
                handler: NotifyHandler::One(*con),
            });
        }
    }
}

impl NetworkBehaviour for Transfer {
    type ProtocolsHandler = OneShotHandler<TransferConfig, Message, HandlerEvent>;
    type OutEvent = TransferEvent;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        Default::default()
    }

    fn addresses_of_peer(&mut self, _peer: &PeerId) -> Vec<Multiaddr> {
        Vec::new()
    }

    fn inject_connection_established(&mut self, peer: &PeerId, con: &ConnectionId, _: &ConnectedPoint) {
        self.cur_id +=1;
        self.peers.insert(self.cur_id,  (*peer,*con));
    }

    fn inject_connected(&mut self, _peer: &PeerId) {
        
    }

    fn inject_disconnected(&mut self, _peer: &PeerId) {
        
    }

    fn inject_event(&mut self, peer: PeerId, con: ConnectionId, hevent: HandlerEvent) {
        use HandlerEvent::*;
        let ev = match hevent {
            
            Rx(Message::Send(msg)) => TransferEvent::Received(peer,con, msg),
            
            Tx => TransferEvent::Nothing,
            
        };
        self.events
            .push_back(NetworkBehaviourAction::GenerateEvent(ev));
    }

    fn poll(
        &mut self,
        _: &mut Context,
        _: &mut impl PollParameters,
    ) -> Poll<NetworkBehaviourAction<Message, Self::OutEvent>> {
        if let Some(event) = self.events.pop_front() {
            Poll::Ready(event)
        } else {
            Poll::Pending
        }
    }
}

/// Transmission between the `OneShotHandler` and the `BroadcastHandler`.
#[derive(Debug)]
pub enum HandlerEvent {
    /// We received a `Message` from a remote.
    Rx(Message),
    Tx,
}

impl From<Message> for HandlerEvent {
    fn from(message: Message) -> Self {
        Self::Rx(message)
    }
}

impl From<()> for HandlerEvent {
    fn from(_: ()) -> Self {
        Self::Tx
    }
}
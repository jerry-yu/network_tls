use crate::network::{CommomNetMsg, SessionID};
use rand;
use std::collections::{BTreeMap, HashSet};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::time::timeout;

#[derive(Debug)]
pub enum ControlInfoMsg {
    Connected(SessionID, String),
    DisConnected(SessionID),
}

pub struct Control {
    not_con_list: HashSet<String>,
    con_list: BTreeMap<SessionID, String>,
    con_info_rx: UnboundedReceiver<ControlInfoMsg>,
    ctl_tx: UnboundedSender<CommomNetMsg>,
}

impl Control {
    pub fn new(
        not_con_list: HashSet<String>,
        con_info_rx: UnboundedReceiver<ControlInfoMsg>,
        ctl_tx: UnboundedSender<CommomNetMsg>,
    ) -> Control {
        Control {
            not_con_list,
            con_info_rx,
            con_list: BTreeMap::new(),
            ctl_tx,
        }
    }

    pub async fn run(&mut self) {
        for url in self.not_con_list.clone() {
            self.ctl_tx.send(CommomNetMsg::ToConnect(url));
        }
        loop {
            match timeout(std::time::Duration::from_secs(18), self.con_info_rx.recv()).await {
                Ok(Some(msg)) => match msg {
                    ControlInfoMsg::Connected(sid, url) => {
                        self.con_list.insert(sid, url.clone());
                        self.not_con_list.remove(&url);
                    }
                    ControlInfoMsg::DisConnected(sid) => {
                        if let Some(url) = self.con_list.remove(&sid) {
                            self.not_con_list.insert(url);
                        }
                    }
                },

                Err(_) => {
                    for url in self.not_con_list.clone() {
                        self.ctl_tx.send(CommomNetMsg::ToConnect(url));
                    }
                }
                _ => break,
            }
        }
    }
}

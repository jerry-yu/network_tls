use crate::network::SessionID;
use std::collections::{BTreeMap, HashSet};
use tokio::sync::mpsc::UnboundedReceiver;

#[derive(Debug)]
pub enum ControlMsg {
    ToConnect(String),
    Connected(SessionID, String),
    DisConnected(SessionID),
}

pub struct Control {
    not_con_list: HashSet<String>,
    con_list: BTreeMap<SessionID, String>,
    con_info_rx: UnboundedReceiver<ControlMsg>,
    ctl_tx: UnboundedReceiver<ControlMsg>,
}

impl Control {
    pub fn new(
        not_con_list: HashSet<String>,
        con_info_rx: UnboundedReceiver<ControlMsg>,
        ctl_tx: UnboundedReceiver<ControlMsg>,
    ) -> Control {
        Control {
            not_con_list,
            con_info_rx,
            con_list: BTreeMap::new(),
            ctl_tx,
        }
    }

    pub async fn run() {}
}

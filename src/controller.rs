use crate::network::{CommomNetMsg, SessionID};
use rand;
use std::collections::{BTreeMap, HashSet};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::time::timeout;
use log::info;

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

    pub async fn run(not_con_list: HashSet<String>,
        con_info_rx: UnboundedReceiver<ControlInfoMsg>,
        ctl_tx: UnboundedSender<CommomNetMsg>,)
     {
         let mut ctl = Control::new(not_con_list,con_info_rx,ctl_tx);
        for url in ctl.not_con_list.iter() {
            ctl.ctl_tx.send(CommomNetMsg::ToConnect(url.clone()));
        }

        loop {
            match timeout(std::time::Duration::from_secs(3), ctl.con_info_rx.recv()).await {
                Ok(Some(msg)) => match msg {
                    
                    ControlInfoMsg::Connected(sid, url) => {
                        info!("controller Connected {} {:?}",sid,url);
                        ctl.con_list.insert(sid, url.clone());
                        ctl.not_con_list.remove(&url);
                    }
                    ControlInfoMsg::DisConnected(sid) => {
                        info!("controller DisConnected {}",sid);
                        if let Some(url) = ctl.con_list.remove(&sid) {
                            ctl.not_con_list.insert(url);
                        }
                    }
                },

                Err(e) => {
                    info!("timeout -- {:?} ,list {:?}",e,ctl.not_con_list);
                    for url in ctl.not_con_list.clone() {
                        ctl.ctl_tx.send(CommomNetMsg::ToConnect(url));
                    }
                }
                _ => break,
            }
        }
    }
}

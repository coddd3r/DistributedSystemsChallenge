use std::collections::HashMap;

use anyhow::Context;
use ds_challenge::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
enum Payload {
    Txn {
        msg_id: usize,
        txn: Vec<Vec<String>>,
    },
    TxnOk {
        msg_id: usize,
        in_reply_to: String,
        txn: Vec<Vec<String>>,
    },
}

struct TxnNode {
    // node: String,
    id: usize,
    // initial_nodes: Vec<String>,
    own_log: HashMap<String, String>,
}

enum InjectedPayload {
    // Gossip,
}

impl Node<(), Payload, InjectedPayload> for TxnNode {
    fn from_init(
        _state: (),
        _init: Init,
        _inject: std::sync::mpsc::Sender<Event<Payload, InjectedPayload>>,
    ) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            // node: init.node_id,
            id: 1,
            // initial_nodes: init.node_ids,
            own_log: HashMap::new(),
        })
    }

    fn handle_input(
        &mut self,
        input: Event<Payload, InjectedPayload>,
        output: &mut std::io::StdoutLock,
    ) -> anyhow::Result<()> {
        match input {
            Event::Injected(_) => {}
            Event::EOF => {}
            Event::Message(input) => {
                eprintln!("received a message: {:?}", &input);
                let mut response = input.derive_response(Some(&mut self.id));
                match response.body.payload {
                    Payload::Txn { msg_id, txn } => {
                        let mut ret_txn: Vec<_> = Vec::new();
                        txn.iter().for_each(|tr| {
                            let mut tr = tr.iter();
                            if let Some(op_name) = tr.next() {
                                if op_name == "w" {
                                    if let (Some(key), Some(msg)) = (tr.next(), tr.next()) {
                                        self.own_log.insert(key.clone(), msg.clone());
                                        ret_txn.push(vec![
                                            op_name.clone(),
                                            key.to_string(),
                                            msg.clone(),
                                        ]);
                                    }
                                }
                                if op_name == "r" {
                                    if let Some(key) = tr.next() {
                                        if let Some(msg) = self.own_log.get(key) {
                                            ret_txn.push(vec![
                                                op_name.clone(),
                                                key.to_string(),
                                                msg.clone(),
                                            ]);
                                        }
                                    }
                                }
                            }
                        });
                        response.body.payload = Payload::TxnOk {
                            msg_id,
                            in_reply_to: response.dest.clone(),
                            txn: ret_txn,
                        };

                        response
                            .send_self(&mut *output)
                            .context("Txb response failed")?;
                    }
                    Payload::TxnOk { .. } => {}
                }
            }
        }
        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    main_loop::<_, TxnNode, _, _>(())
}

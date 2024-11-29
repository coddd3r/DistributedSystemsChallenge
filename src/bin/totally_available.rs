use std::collections::HashMap;

use anyhow::Context;
use ds_challenge::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
enum TxnTypes {
    TxnCode(String),
    TxnKey(usize),
    TxnMessage(usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
enum Payload {
    Txn {
        msg_id: usize,
        txn: Vec<Vec<TxnTypes>>,
    },
    TxnOk {
        msg_id: usize,
        in_reply_to: String,
        txn: Vec<Vec<TxnTypes>>,
    },
}

struct TxnNode {
    node: String,
    id: usize,
    initial_nodes: Vec<String>,
    own_log: HashMap<usize, usize>,
}

enum InjectedPayload {
    // Gossip,
}

impl Node<(), Payload, InjectedPayload> for TxnNode {
    fn from_init(
        _state: (),
        init: Init,
        _inject: std::sync::mpsc::Sender<Event<Payload, InjectedPayload>>,
    ) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            node: init.node_id,
            id: 1,
            initial_nodes: init.node_ids,
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
                let mut response = input.derive_response(Some(&mut self.id));
                match response.body.payload {
                    Payload::Txn { msg_id, txn } => {
                        let mut ret_txn: Vec<_> = Vec::new();
                        txn.iter().for_each(|tr| {
                            let mut tr = tr.iter();
                            if let TxnTypes::TxnCode(op_name) = tr
                                .next()
                                .expect("There should be a transaction shortcut 'r' or 'w'")
                            {
                                if op_name == "w" {
                                    if let (
                                        Some(TxnTypes::TxnKey(key)),
                                        Some(TxnTypes::TxnMessage(msg)),
                                    ) = (tr.next(), tr.next())
                                    {
                                        self.own_log.insert((*key).clone(), (*msg).clone());
                                        ret_txn.push(vec![
                                            TxnTypes::TxnCode(op_name.clone()),
                                            TxnTypes::TxnKey(*key),
                                            TxnTypes::TxnMessage(*msg),
                                        ]);
                                    }
                                }
                                if op_name == "r" {
                                    if let Some(TxnTypes::TxnKey(key)) = tr.next() {
                                        let ret = self.own_log.entry(*key).or_insert(0);
                                        ret_txn.push(vec![
                                            TxnTypes::TxnCode(op_name.clone()),
                                            TxnTypes::TxnKey(*key),
                                            TxnTypes::TxnMessage(ret.clone()),
                                        ])
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

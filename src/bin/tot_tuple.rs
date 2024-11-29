use ds_challenge::*;

use anyhow::Context;
use serde::{Deserialize, Serialize};
// use serde_with::serde_as;
use std::{collections::HashMap, fmt::Debug, usize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
enum Payload {
    Txn {
        txn: Vec<(String, usize, Option<usize>)>,
    },
    TxnOk {
        txn: Vec<(String, usize, usize)>,
    },
}

struct TxnNode {
    id: usize,
    own_log: HashMap<usize, usize>,
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
                let mut response = input.derive_response(Some(&mut self.id));
                match response.body.payload {
                    Payload::Txn { txn } => {
                        let mut ret_txn: Vec<_> = Vec::new();

                        txn.clone().into_iter().for_each(|op| {
                            let (op_name, key, msg) = op;
                            if op_name == "w" {
                                if let Some(msg) = msg {
                                    self.own_log.insert(key, msg);
                                    ret_txn.push((op_name, key, msg));
                                }
                            }
                        });

                        txn.into_iter().for_each(|op| {
                            let (op_name, key, _    ) = op;
                            if op_name == "r" {
                                let ret = self.own_log.entry(key).or_insert(0);
                                ret_txn.push((op_name, key, ret.clone()));
                            }
                        });

                        response.body.payload = Payload::TxnOk { txn: ret_txn };

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

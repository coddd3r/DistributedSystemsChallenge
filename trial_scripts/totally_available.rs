use ds_challenge::*;

use anyhow::Context;
use serde::{Deserialize, Serialize};
// use serde_with::serde_as;
use std::{collections::HashMap, fmt::Debug, usize};

// #[derive(Debug, Clone)]
#[derive(Debug, Clone, Serialize, Deserialize)]
// #[serde(untagged)]
enum TxnTypes {
    String(String),
    Usize(usize),
    Null,
}

// impl Serialize for TxnTypes {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         match self {
//             TxnTypes::String(c) => c.serialize(serializer),
//             TxnTypes::Usize(k) => k.serialize(serializer),
//             TxnTypes::Null => {}
//         }
//     }
// }

// impl<'de> Deserialize<'de> for TxnTypes {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         let value: String = Deserialize::deserialize(deserializer)?;

//         match value.parse::<usize>() {
//             Ok(val) => Ok(TxnTypes::Usize(val)),
//             Err(_) => Ok(TxnTypes::String(value)),
//         }
//     }
// }

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
    // node: String,
    id: usize,
    // initial_nodes: Vec<String>,
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
                    Payload::Txn { msg_id, txn } => {
                        let mut ret_txn: Vec<_> = Vec::new();
                        txn.iter().for_each(|tr| {
                            let mut tr = tr.iter();
                            if let TxnTypes::String(op_name) = tr
                                .next()
                                .expect("There should be a transaction shortcut 'r' or 'w'")
                            {
                                if op_name == "w" {
                                    if let (
                                        Some(TxnTypes::Usize(key)),
                                        Some(TxnTypes::Usize(msg)),
                                    ) = (tr.next(), tr.next())
                                    {
                                        self.own_log.insert((*key).clone(), (*msg).clone());
                                        ret_txn.push(vec![
                                            TxnTypes::String(op_name.clone()),
                                            TxnTypes::Usize(*key),
                                            TxnTypes::Usize(*msg),
                                        ]);
                                    }
                                }
                                if op_name == "r" {
                                    if let Some(TxnTypes::Usize(key)) = tr.next() {
                                        let ret = self.own_log.entry(*key).or_insert(0);
                                        ret_txn.push(vec![
                                            TxnTypes::String(op_name.clone()),
                                            TxnTypes::Usize(*key),
                                            TxnTypes::Usize(ret.clone()),
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

/*
    Challenge 6:Totally-available
    passes all tests 6a, 6b, 6c
*/

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
    Gossip {
        gossiped_log: HashMap<usize, usize>,
    },
}

struct TxnNode {
    // node: String,
    id: usize,
    own_log: HashMap<usize, usize>,
    // initial_nodes: Vec<String>,
}

enum InjectedPayload {
    // Gossip,
}

impl Node<(), Payload, InjectedPayload> for TxnNode {
    fn from_init(
        _state: (),
        _init: Init,
        _tx: std::sync::mpsc::Sender<Event<Payload, InjectedPayload>>,
    ) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        // std::thread::spawn(move || loop {
        //     std::thread::sleep(Duration::from_millis(500));
        //     if let Err(_) = tx.send(Event::Injected(InjectedPayload::Gossip)) {
        //         break;
        //     }
        // });

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
            // Event::Injected(payload) => match payload {
            // InjectedPayload::Gossip => {
            //     eprintln!("GOSSIPING");
            //     for n in &self.initial_nodes {
            //         let msg: Message<Payload> = Message {
            //             src: self.node.clone(),
            //             dest: n.clone(),
            //             body: Body {
            //                 id: Some(self.id),
            //                 in_reply_to: None,
            //                 payload: Payload::Gossip {
            //                     gossiped_log: self.own_log.clone(),
            //                 },
            //             },
            //         };
            //         eprintln!("sending gossip with record{:?}", &self.own_log);
            //         msg.send_self(&mut *output)
            //             .context("failed to gossip replicated record in kafka/replicated")?;
            //         self.id += 1;
            //     }
            // }
            // },
            Event::EOF => {}
            Event::Message(input) => {
                let mut response = input.derive_response(Some(&mut self.id));
                match response.body.payload {
                    Payload::Gossip { gossiped_log } => {
                        eprintln!("got into gossip");
                        gossiped_log.into_iter().for_each(|(k, v)| {
                            let own_val = self.own_log.entry(k).or_insert(v);
                            if v > *own_val {
                                *own_val = v;
                                eprintln!("making changes from gossip");
                            }
                        })
                    }

                    Payload::Txn { txn } => {
                        let mut ret_txn: Vec<_> = Vec::new();

                        txn.clone().into_iter().for_each(|op| {
                            let (op_name, key, msg) = op;
                            if op_name == "w" {
                                if let Some(msg) = msg {
                                    self.own_log.insert(key, msg);
                                    ret_txn.push((op_name, key, msg));
                                }
                            } else if op_name == "r" {
                                let ret = self.own_log.entry(key).or_insert(0);
                                ret_txn.push((op_name, key, ret.clone()));
                            }
                        });

                        //NO NEED TO COMMIT WRITES BEFORE READS YET
                        // txn.into_iter().for_each(|op| {
                        //     let (op_name, key, _) = op;
                        //     if op_name == "r" {
                        //         let ret = self.own_log.entry(key).or_insert(0);
                        //         ret_txn.push((op_name, key, ret.clone()));
                        //     }
                        // });

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

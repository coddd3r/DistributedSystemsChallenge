use std::{
    collections::{HashMap, HashSet},
    str,
    time::Duration,
};

use anyhow::Context;
use ds_challenge::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum Payload {
    Add {
        delta: usize,
    },
    AddOk,
    Read,
    ReadOk {
        value: usize,
    },
    Gossip {
        latest_values: HashMap<String, usize>,
    },
}
enum InjectedPayload {
    Gossip,
}

struct CounterNode {
    node: String,
    id: usize,
    value: usize,
    initial_nodes: HashSet<String>,
    latest_values: HashMap<String, usize>,
}

impl Node<(), Payload, InjectedPayload> for CounterNode {
    fn from_init(
        _state: (),
        init: Init,
        tx: std::sync::mpsc::Sender<Event<Payload, InjectedPayload>>,
    ) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        //add thread to gossip lates counter value
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_millis(100));
            if let Err(_) = tx.send(Event::Injected(InjectedPayload::Gossip)) {
                break;
            }
        });
        //create node to keep track of own_details and all current known nodes
        Ok(Self {
            initial_nodes: init
                .node_ids
                .clone()
                .into_iter()
                .filter(|n| n != &init.node_id)
                .collect(),
            node: init.node_id,
            id: 1,
            value: 0,
            latest_values: init.node_ids.into_iter().map(|nid| (nid, 0)).collect(),
        })
    }

    fn handle_input(
        &mut self,
        input: Event<Payload, InjectedPayload>,
        output: &mut std::io::StdoutLock,
    ) -> anyhow::Result<()> {
        match input {
            Event::Message(input) => {
                let mut response = input.derive_response(Some(&mut self.id));
                match response.body.payload {
                    Payload::Gossip { latest_values } => {
                        // if self.node == response.src {
                        //     eprintln!(
                        //         "GOSSIP TO SELF!!?? self:{}, from:{}, expected dest:{} all_nodes:{:?}, gossiped nodes from src:{:?}",
                        //         &self.node,
                        //         &response.src,
                        //         &response.dest,
                        //         &self.latest_values,
                        //         &latest_values,
                        //     );
                        // }
                        for key in latest_values.keys() {
                            let Some(val) = latest_values.get(key) else {
                                panic!("erroneous key in know_latest");
                            };
                            let own_val =
                                self.latest_values.entry(key.clone()).or_insert(val.clone());
                            if val > own_val {
                                *own_val = *val;
                            }
                        }
                    }
                    Payload::Add { delta } => {
                        response.body.payload = Payload::AddOk;
                        self.value += delta;
                        response
                            .send_self(&mut *output)
                            .context("add delta failure")?;

                        *self.latest_values.entry(self.node.clone()).or_insert(0) += delta;
                    }
                    Payload::AddOk => {}

                    Payload::Read => {
                        let cumulative: usize = self.latest_values.clone().into_values().sum();
                        response.body.payload = Payload::ReadOk { value: cumulative };
                        response.send_self(&mut *output).context("read failure")?
                    }

                    Payload::ReadOk { .. } => {}
                }
            }

            Event::Injected(payload) => match payload {
                InjectedPayload::Gossip => {
                    for n in &self.initial_nodes {
                        if n == &self.node {
                            eprintln!(
                                "found self! {n}, {}, curr nodes: {:?}",
                                &self.node, self.initial_nodes
                            );
                            // continue;
                        }

                        let msg: Message<Payload> = Message {
                            src: self.node.clone(),
                            dest: n.clone(),
                            body: Body {
                                id: Some(self.id),
                                in_reply_to: None,
                                payload: Payload::Gossip {
                                    // latest: self.value,
                                    latest_values: self.latest_values.clone(),
                                },
                            },
                        };
                        msg.send_self(&mut *output)
                            .context(format!("gossip to {n}"))?;
                        self.id += 1;
                    }
                }
            },
            //TODO: handle EOF
            Event::EOF => {}
        }

        Ok(())
    }
}
fn main() -> anyhow::Result<()> {
    //'_' represent unused state, node, Payload and InjectedPayload generics for <S, N, P>
    main_loop::<_, CounterNode, _, _>(())
}

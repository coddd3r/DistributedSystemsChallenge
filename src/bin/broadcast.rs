/*
    broadcast challenge 3a,b,c,d,e
*/
use ds_challenge::*;
use std::{
    collections::{HashMap, HashSet},
    io::StdoutLock,
    time::Duration,
};

use anyhow::Context;
use serde::{Deserialize, Serialize};

//what type of message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum Payload {
    Broadcast {
        message: usize,
    },
    BroadcastOk,
    Read,
    ReadOk {
        messages: HashSet<usize>,
    },
    Topology {
        topology: HashMap<String, Vec<String>>,
    },
    TopologyOk,
    Gossip {
        seen: HashSet<usize>,
    },
}

enum InjectedPayload {
    Gossip,
}
//node representing broadcast
struct BroadcastNode {
    node: String,
    id: usize,
    messages: HashSet<usize>,
    //known: map of node identifier i.e broadcastnode.node, to all the messages we received from it
    //i.e all the messages we're certain they know of
    known: HashMap<String, HashSet<usize>>,
    neighbourhood: Vec<String>,
}

//handle basic Generate responses
impl Node<(), Payload, InjectedPayload> for BroadcastNode {
    fn from_init(
        _state: (),
        init: Init,
        tx: std::sync::mpsc::Sender<Event<Payload, InjectedPayload>>,
    ) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        //generate gossip events susign a thread
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_millis(300));
            if let Err(_) = tx.send(Event::Injected(InjectedPayload::Gossip)) {
                break;
            }
        });
        Ok(Self {
            node: init.node_id,
            id: 1,
            messages: HashSet::new(),
            known: init
                .node_ids
                .into_iter()
                .map(|nid| (nid, HashSet::new()))
                .collect(),
            neighbourhood: Vec::new(),
        })
    }

    fn handle_input(
        &mut self,
        input: ds_challenge::Event<Payload, InjectedPayload>,
        output: &mut StdoutLock,
    ) -> anyhow::Result<()> {
        match input {
            Event::Message(input) => {
                let mut response = input.derive_response(Some(&mut self.id));
                //for each response type, set payload to enum variant and write response to stdout
                match response.body.payload {
                    Payload::Gossip { seen } => {
                        //add the received gossip messages to the hashset at key of node n
                        //i.e the nodes we know n knows
                        self.known
                            .get_mut(&response.dest)
                            .expect("gossip from None node")
                            .extend(seen.iter().copied());
                        self.messages.extend(seen);
                        //gossip is done periodically and does not have a response
                    }
                    Payload::Broadcast { message } => {
                        self.messages.insert(message);
                        response.body.payload = Payload::BroadcastOk;
                        response
                            .send_self(&mut *output)
                            .context("respond to broadcast")?;
                    }

                    Payload::Read => {
                        response.body.payload = Payload::ReadOk {
                            messages: self.messages.clone(),
                        };
                        response.send_self(&mut *output).context("reply to Read")?;
                    }

                    Payload::Topology { mut topology } => {
                        self.neighbourhood = topology
                            .remove(&self.node)
                            .unwrap_or_else(|| panic!("no topology for noe {}", self.node));
                        response.body.payload = Payload::TopologyOk;
                        response
                            .send_self(&mut *output)
                            .context("reply to topology message")?;
                    }

                    Payload::TopologyOk { .. }
                    | Payload::BroadcastOk { .. }
                    | Payload::ReadOk { .. } => {}
                }
            }
            Event::Injected(payload) => match payload {
                InjectedPayload::Gossip => {
                    for n in &self.neighbourhood {
                        let known_messages = &self.known[n];
                        let notify_of = self
                            .messages
                            .iter()
                            .copied()
                            .filter(|m| !known_messages.contains(m))
                            .collect();

                        let msg: Message<Payload> = Message {
                            src: self.node.clone(),
                            dest: n.clone(),
                            body: Body {
                                id: Some(self.id),
                                in_reply_to: None,
                                payload: Payload::Gossip { seen: notify_of },
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
    main_loop::<_, BroadcastNode, _, _>(())
}

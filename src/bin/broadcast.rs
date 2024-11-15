use ds_challenge::*;
use std::{
    collections::{HashMap, HashSet},
    io::StdoutLock,
    time::Duration,
};
// use ulid::Ulid;

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
        //derive a message response with the prev held payload
        // let Event::Message(input) = input else {
        //     panic!("got unexpected injected event")
        // };

        match input {
            Event::Message(input) => {
                let mut response = input.derive_response(Some(&mut self.id));
                //for each response type, set payload to enum variant and write response to stdout
                match response.body.payload {
                    Payload::Gossip { seen } => {
                        self.messages.extend(seen);
                    }
                    Payload::Broadcast { message } => {
                        self.messages.insert(message);
                        response.body.payload = Payload::BroadcastOk;
                        response
                            .send_self(&mut *output)
                            .context("respond to broadcast")?;
                    }
                    Payload::BroadcastOk { .. } => {}

                    Payload::Read => {
                        response.body.payload = Payload::ReadOk {
                            messages: self.messages.clone(),
                        };
                        response.send_self(&mut *output).context("reply to Read")?;
                    }
                    Payload::ReadOk { .. } => {}

                    Payload::Topology { mut topology } => {
                        self.neighbourhood = topology
                            .remove(&self.node)
                            .unwrap_or_else(|| panic!("no topology for noe {}", self.node));
                        response.body.payload = Payload::TopologyOk;
                        response
                            .send_self(&mut *output)
                            .context("reply to topology message")?;
                    }
                    Payload::TopologyOk { .. } => {}
                }
            }
            Event::Injected(payload) => match payload {
                InjectedPayload::Gossip => {
                    for n in &self.neighbourhood {
                        let known_messages = &self.known[n];
                        let msg: Message<Payload> = Message {
                            src: self.node.clone(),
                            dest: n.clone(),
                            body: Body {
                                id: Some(self.id),
                                in_reply_to: None,
                                payload: Payload::Gossip {
                                    seen: self
                                        .messages
                                        .iter()
                                        .copied()
                                        .filter(|m| !known_messages.contains(m))
                                        .collect(),
                                },
                            },
                        };
                        msg.send_self(&mut *output)
                            .context(format!("gossip to {n}"))?;
                        self.id += 1;
                    }
                }
            },
            Event::EOF => {}
        }

        Ok(())
    }
}
fn main() -> anyhow::Result<()> {
    //'_' represent unused state and payload generics for <S, N, P>
    main_loop::<_, BroadcastNode, _, _>(())
}

use ds_challenge::*;

use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::Duration};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
enum Payload {
    Send {
        key: String,
        msg: usize,
    },
    SendOk {
        offset: usize,
    },
    Poll {
        offsets: HashMap<String, usize>,
    },
    PollOk {
        msgs: HashMap<String, Vec<(usize, usize)>>,
    },
    CommitOffsets {
        offsets: HashMap<String, usize>,
    },
    CommitOffsetsOk,
    ListCommittedOffsets {
        keys: Vec<String>,
    },
    ListCommittedOffsetsOk {
        offsets: HashMap<String, usize>,
    },
    Gossip {
        gossip_max: HashMap<String, usize>,
    },
}

enum InjectedPayload {
    Gossip,
}

struct RecordNode {
    node: String,
    id: usize,
    initial_nodes: Vec<String>,
    committed_offsets: HashMap<String, usize>,
    current_max: HashMap<String, usize>,
}

impl Node<(), Payload, InjectedPayload> for RecordNode {
    fn from_init(
        _state: (),
        init: Init,
        tx: std::sync::mpsc::Sender<Event<Payload, InjectedPayload>>,
    ) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_millis(250));
            if let Err(_) = tx.send(Event::Injected(InjectedPayload::Gossip)) {
                break;
            }
        });

        Ok(Self {
            node: init.node_id,
            id: 0,
            initial_nodes: init.node_ids,
            committed_offsets: HashMap::new(),
            current_max: HashMap::new(),
        })
    }

    fn handle_input(
        &mut self,
        input: Event<Payload, InjectedPayload>,
        output: &mut std::io::StdoutLock,
    ) -> anyhow::Result<()> {
        match input {
            Event::EOF => {}

            Event::Injected(payload) => match payload {
                InjectedPayload::Gossip => {
                    //eprintln!("GOSSIPING");
                    for n in &self.initial_nodes {
                        let msg: Message<Payload> = Message {
                            src: self.node.clone(),
                            dest: n.clone(),
                            body: Body {
                                id: Some(self.id),
                                in_reply_to: None,
                                payload: Payload::Gossip {
                                    gossip_max: self.current_max.clone(),
                                },
                            },
                        };
                        //eprintln!("sending gossip with maxes{:?}", &self.current_max);
                        msg.send_self(&mut *output)
                            .context("failed to gossip replicated record in kafka/replicated")?;
                        self.id += 1;
                    }
                }
            },

            Event::Message(input) => {
                let mut response = input.derive_response(Some(&mut self.id));
                match response.body.payload {
                    Payload::Gossip { gossip_max } => {
                        //eprintln!("received gossip, with max_map {:?}", &gossip_max);
                        //eprintln!("current maxes before merge {:?}", &self.current_max);
                        for (log_key, max_msg) in gossip_max {
                            let own_max = self.current_max.entry(log_key).or_insert(0);
                            *own_max = max_msg.max(*own_max);
                        }
                        //eprintln!("maxes after gossip{:?}", &self.current_max);
                    }

                    Payload::Send { key, msg } => {
                        //eprintln!("RECEIVED SEND for key {key}, message:{msg}");
                        let mut send_offset = &key.parse::<usize>().unwrap() * 10000;
                        self.current_max.insert(key, msg);
                        send_offset += msg;
                        //eprintln!("current max after adding from send {:?}", self.current_max);
                        //eprintln!("sending send_ok with offset {}", &send_offset);
                        response.body.payload = Payload::SendOk {
                            offset: send_offset,
                        };

                        response
                            .send_self(&mut *output)
                            .context("failed to respond to send request in replicated record")?;
                    }

                    Payload::Poll { offsets } => {
                        //eprintln!("RECEIVED POLL with dictionary:{:?}", &offsets);
                        //eprintln!("current maxes:{:?}", &self.current_max);
                        let mut ret_map: HashMap<String, Vec<(usize, usize)>> = HashMap::new();

                        for (k, val) in offsets {
                            let v = val % 10000;
                            //eprintln!("in poll offset loop, current key:{k}, value:{v}");

                            let Some(key_max) = self.current_max.get(&k) else {
                                //eprintln!("KEY: {k} NOT FOUND in {:?}", self.current_max);
                                continue;
                            };
                            //eprintln!("in POLL, key max before:{:?}", &key_max);
                            let key_offset = k.parse::<usize>().unwrap() * 10000;
                            let mut fin_set: Vec<_> = Vec::new();

                            let v = v.max(1);
                            for i in v..*key_max + 1 {
                                fin_set.push((i + key_offset, i));
                            }

                            //eprintln!("POLL result: after:{:?}", &fin_set);
                            ret_map.insert(k, fin_set);
                        }

                        //eprintln!("RESPONDING to a poll with map: {:?}", &ret_map);
                        response.body.payload = Payload::PollOk { msgs: ret_map };
                        response
                            .send_self(&mut *output)
                            .context("failed to respond to poll request")?;
                    }

                    Payload::CommitOffsets { offsets } => {
                        //eprintln!("updating committed offsets: {:?}", offsets);
                        for (k, v) in offsets {
                            self.committed_offsets.insert(k, v);
                        }
                        //eprintln!(
                        //     "recorded offsets after commit: {:?}",
                        //     self.committed_offsets
                        // );
                        response.body.payload = Payload::CommitOffsetsOk;
                        response
                            .send_self(&mut *output)
                            .context("failed to respond to commit offset")?;
                    }

                    Payload::ListCommittedOffsets { keys } => {
                        //eprintln!("received LIST commit offsets request with keys {:?}", &keys);
                        //eprintln!("committed offsets before:{:?}", &self.committed_offsets);
                        let mut ret_map: HashMap<String, usize> = HashMap::new();
                        for key in keys {
                            if let Some(val) = self.committed_offsets.get(&key) {
                                ret_map.insert(key, val.clone());
                            } else {
                                //eprintln!("KEY NOT FOUND IN LIST COMMITTED OFFSETS");
                                continue;
                            }
                        }
                        //eprintln!("returning committed offsets with map {:?}", ret_map);
                        response.body.payload =
                            Payload::ListCommittedOffsetsOk { offsets: ret_map };
                        response
                            .send_self(&mut *output)
                            .context("failed to respond to ListCommitOffset")?;
                    }

                    Payload::SendOk { .. } => {}
                    Payload::PollOk { .. } => {}
                    Payload::CommitOffsetsOk => {}
                    Payload::ListCommittedOffsetsOk { .. } => {}
                }
            }
        }

        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    main_loop::<_, RecordNode, _, _>(())
}

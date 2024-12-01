/*
    challenge 5: kafka style log
*/

use ds_challenge::*;

use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

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
        history: HashMap<String, HashSet<usize>>,
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
    record: HashMap<String, HashSet<usize>>,
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
            record: HashMap::new(),
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
                    for n in &self.initial_nodes {
                        let msg: Message<Payload> = Message {
                            src: self.node.clone(),
                            dest: n.clone(),
                            body: Body {
                                id: Some(self.id),
                                in_reply_to: None,
                                payload: Payload::Gossip {
                                    history: self.record.clone(),
                                },
                            },
                        };
                        msg.send_self(&mut *output)
                            .context("failed to gossip replicated record in kafka/replicated")?;
                        self.id += 1;
                    }
                }
            },

            Event::Message(input) => {
                let mut response = input.derive_response(Some(&mut self.id));
                match response.body.payload {
                    Payload::Gossip { history } => {
                        for (record_key, gossip_vec) in history {
                            let own_vec = self.record.entry(record_key).or_insert(HashSet::new());
                            own_vec.extend(gossip_vec);
                        }
                    }

                    Payload::Send { key, msg } => {
                        let mut send_offset = &key.parse::<usize>().unwrap() * 10000;
                        let key_set = self.record.entry(key).or_insert(HashSet::new());
                        key_set.insert(msg);
                        send_offset += msg;
                        response.body.payload = Payload::SendOk {
                            offset: send_offset,
                        };

                        response
                            .send_self(&mut *output)
                            .context("failed to respond to send request in replicated record")?;
                    }

                    Payload::Poll { offsets } => {
                        let mut ret_map: HashMap<String, Vec<(usize, usize)>> = HashMap::new();
                        for (k, v) in offsets {
                            let v = v % 10000;

                            let Some(key_set) = self.record.get(&k) else {
                                continue;
                            };
                            let mut ret_set: Vec<usize> =
                                key_set.clone().into_iter().filter(|m| *m >= v).collect();
                            ret_set.sort();
                            let key_offset = k.parse::<usize>().unwrap() * 10000;
                            let mut fin_set: Vec<_> = Vec::new();
                            if ret_set.is_empty() {
                                continue;
                            }
                            let up = ret_set.iter().max().unwrap();
                            let v = v.max(1);
                            for i in v..=*up {
                                fin_set.push((i + key_offset, i));
                            }

                            ret_map.insert(k, fin_set);
                        }

                        response.body.payload = Payload::PollOk { msgs: ret_map };
                        response
                            .send_self(&mut *output)
                            .context("failed to respond to poll request")?;
                    }

                    Payload::CommitOffsets { offsets } => {
                        for (k, v) in offsets {
                            self.committed_offsets.insert(k, v);
                        }

                        response.body.payload = Payload::CommitOffsetsOk;
                        response
                            .send_self(&mut *output)
                            .context("failed to respond to commit offset")?;
                    }

                    Payload::ListCommittedOffsets { keys } => {
                        let mut ret_map: HashMap<String, usize> = HashMap::new();
                        for key in keys {
                            if let Some(val) = self.committed_offsets.get(&key) {
                                ret_map.insert(key, val.clone());
                            } else {
                                continue;
                            }
                        }

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

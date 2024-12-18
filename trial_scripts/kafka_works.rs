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
        // msgs: HashMap<String, Vec<usize>>,
        // msgs: HashMap<String, HashSet<(usize, usize)>>,
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
        // history: HashMap<String, Vec<(usize, usize)>>,
        history: HashMap<String, HashSet<usize>>,
        // history: HashMap<String, Vec<usize>>,
    },
}

enum InjectedPayload {
    Gossip,
}

struct RecordNode {
    node: String,
    id: usize,
    // record: HashMap<String, usize>,
    // record: HashMap<String, Vec<(usize, usize)>>,
    // record: HashMap<String, Vec<usize>>,
    record: HashMap<String, HashSet<usize>>,
    // count: usize,
    initial_nodes: Vec<String>,
    committed_offsets: HashMap<String, usize>,
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
            record: HashMap::new(),
            // count: 1,
            committed_offsets: HashMap::new(),
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
                    eprintln!("GOSSIPING");
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
                        eprintln!("sending gossip with record{:?}", &self.record);
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
                        eprintln!("received gossip, with record {:?}", &history);
                        eprintln!("current record before merge {:?}", &self.record);
                        for (record_key, gossip_vec) in history {
                            let own_vec = self.record.entry(record_key).or_insert(HashSet::new());
                            own_vec.extend(gossip_vec);
                        }
                        eprintln!("record after gossip{:?}", &self.record);
                    }

                    Payload::Send { key, msg } => {
                        eprintln!("RECEIVED SEND for key {key}, message:{msg}");
                        let mut send_offset = &key.parse::<usize>().unwrap() * 10000;
                        let key_set = self.record.entry(key).or_insert(HashSet::new());
                        key_set.insert(msg);
                        send_offset += msg;
                        eprintln!("current record after adding from send {:?}", self.record);
                        eprintln!("sending send_ok with offset {}", &send_offset);
                        response.body.payload = Payload::SendOk {
                            offset: send_offset,
                        };

                        response
                            .send_self(&mut *output)
                            .context("failed to respond to send request in replicated record")?;
                    }

                    Payload::Poll { offsets } => {
                        eprintln!("RECEIVED POLL with dictionary:{:?}", &offsets);
                        eprintln!("current record:{:?}", &self.record);
                        let mut ret_map: HashMap<String, Vec<(usize, usize)>> = HashMap::new();
                        for (k, v) in offsets {
                            let v = v % 10000;
                            eprintln!("in poll offset loop, current key:{k}, value:{v}");

                            let Some(key_set) = self.record.get(&k) else {
                                eprintln!("KEY: {k} NOT FOUND in {:?}", self.record);
                                continue;
                            };
                            eprintln!("in POLL, key set before:{:?}", &key_set);
                            let mut ret_set: Vec<usize> =
                                key_set.clone().into_iter().filter(|m| *m >= v).collect();
                            ret_set.sort();

                            let key_offset = k.parse::<usize>().unwrap() * 10000;
                            let ret_set = ret_set
                                .into_iter()
                                .enumerate()
                                .map(|(_, m)| (m + key_offset, m))
                                .collect();
                            eprintln!("POLL result: after:{:?}", &ret_set);
                            ret_map.insert(k, ret_set);
                        }
                        eprintln!("sending a poll with map: {:?}", &ret_map);
                        response.body.payload = Payload::PollOk { msgs: ret_map };
                        response
                            .send_self(&mut *output)
                            .context("failed to respond to poll request")?;
                    }

                    Payload::CommitOffsets { offsets } => {
                        eprintln!("updating committed offsets: {:?}", offsets);
                        for (k, v) in offsets {
                            self.committed_offsets.insert(k, v);
                        }
                        eprintln!(
                            "recorded offsets after commit: {:?}",
                            self.committed_offsets
                        );
                        response.body.payload = Payload::CommitOffsetsOk;
                        response
                            .send_self(&mut *output)
                            .context("failed to respond to commit offset")?;
                    }

                    Payload::ListCommittedOffsets { keys } => {
                        eprintln!("received LIST commit offsets request with keys {:?}", &keys);
                        let mut ret_map: HashMap<String, usize> = HashMap::new();
                        for key in keys {
                            if let Some(val) = self.committed_offsets.get(&key) {
                                ret_map.insert(key, val.clone());
                            } else {
                                eprintln!("KEY NOT FOUND IN LIST COMMITTED OFFSETS");
                                continue;
                            }
                        }
                        eprintln!("returning committed offsets with map {:?}", ret_map);
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

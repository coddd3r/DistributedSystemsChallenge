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

struct LogNode {
    node: String,
    id: usize,
    // log: HashMap<String, usize>,
    // log: HashMap<String, Vec<(usize, usize)>>,
    // log: HashMap<String, Vec<usize>>,
    log: HashMap<String, HashSet<usize>>,
    // count: usize,
    initial_nodes: Vec<String>,
    committed_offsets: HashMap<String, usize>,
}

impl Node<(), Payload, InjectedPayload> for LogNode {
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
            log: HashMap::new(),
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
                    //eprintln!("GOSSIPING");
                    for n in &self.initial_nodes {
                        let msg: Message<Payload> = Message {
                            src: self.node.clone(),
                            dest: n.clone(),
                            body: Body {
                                id: Some(self.id),
                                in_reply_to: None,
                                payload: Payload::Gossip {
                                    history: self.log.clone(),
                                },
                            },
                        };
                        //eprintln!("sending gossip with log{:?}", &self.log);
                        msg.send_self(&mut *output)
                            .context("failed to gossip replicated log in kafka/replicated")?;
                        self.id += 1;
                    }
                }
            },

            Event::Message(input) => {
                let mut response = input.derive_response(Some(&mut self.id));
                match response.body.payload {
                    Payload::Gossip { history } => {
                        //eprintln!("received gossip, with log {:?}", &history);
                        //eprintln!("current log before merge {:?}", &self.log);
                        for (log_key, gossip_vec) in history {
                            let own_vec = self.log.entry(log_key).or_insert(HashSet::new());
                            // let mut own_vec = own_vec.clone();
                            own_vec.extend(gossip_vec);
                            // own_vec.sort();
                        }
                        //eprintln!("log after gossip{:?}", &self.log);
                    }

                    Payload::Send { key, msg } => {
                        //eprintln!("RECEIVED SEND for key {key}, message:{msg}");
                        let mut send_offset = &key.parse::<usize>().unwrap() * 10000;
                        let key_set = self.log.entry(key).or_insert(HashSet::new());
                        key_set.insert(msg);
                        //TODO: find a way to avoid getting length every time
                        send_offset += key_set.len();
                        //eprintln!("current log after adding from send {:?}", self.log);
                        //eprintln!("sending send_ok with offset {}", &send_offset);
                        response.body.payload = Payload::SendOk {
                            offset: send_offset,
                        };

                        response
                            .send_self(&mut *output)
                            .context("failed to respond to send request in replicated log")?;
                    }

                    Payload::Poll { offsets } => {
                        //eprintln!("RECEIVED POLL with dictionary:{:?}", &offsets);
                        //eprintln!("current log:{:?}", &self.log);
                        // let mut ret_map: HashMap<String, HashSet<(usize, usize)>> = HashMap::new();
                        let mut ret_map: HashMap<String, Vec<(usize, usize)>> = HashMap::new();
                        // let mut ret_map: HashMap<String, Vec<usize>> = HashMap::new();
                        for (k, v) in offsets {
                            let v = v % 10000;
                            //eprintln!("in poll offset loop, current key:{k}, value:{v}");

                            let Some(key_set) = self.log.get(&k) else {
                                //eprintln!("KEY: {k} NOT FOUND in {:?}", self.log);
                                continue;
                            };
                            //eprintln!("in POLL, key set before:{:?}", &key_set);
                            // let ret_set: Vec<usize> = key_set.clone().drain(v..).collect();
                            let mut ret_set: Vec<usize> =
                                key_set.clone().into_iter().filter(|m| *m >= v).collect();
                            ret_set.sort();

                            let ret_set = ret_set
                                .into_iter()
                                .enumerate()
                                .map(|(o, m)| (o + k.parse::<usize>().unwrap() * 10000, m))
                                .collect();
                            //eprintln!("POLL result: after:{:?}", &ret_set);
                            ret_map.insert(k, ret_set);
                        }
                        //eprintln!("sending a poll with map: {:?}", &ret_map);
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
                        let mut ret_map: HashMap<String, usize> = HashMap::new();
                        for key in keys {
                            // if !self.committed_offsets.contains_key(&key) {continue;}
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
    main_loop::<_, LogNode, _, _>(())
}

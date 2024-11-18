use ds_challenge::*;

use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

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
}

struct LogNode {
    // node: String,
    id: usize,
    // log: HashMap<String, usize>,
    log: HashMap<String, HashSet<(usize, usize)>>,
    count: usize,
    committed_offsets: HashMap<String, usize>,
}

impl Node<(), Payload> for LogNode {
    fn from_init(
        _state: (),
        _init: Init,
        _inject: std::sync::mpsc::Sender<Event<Payload, ()>>,
    ) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            // node: init.node_id,
            id: 0,
            log: HashMap::new(),
            count: 0,
            committed_offsets: HashMap::new(),
        })
    }

    fn handle_input(
        &mut self,
        input: Event<Payload, ()>,
        output: &mut std::io::StdoutLock,
    ) -> anyhow::Result<()> {
        match input {
            Event::EOF => {}
            Event::Injected(..) => {
                panic!("UNEXEPECTED INJECTED IN REPLICATED LOG")
            }
            Event::Message(input) => {
                let mut response = input.derive_response(Some(&mut self.id));
                match response.body.payload {
                    Payload::Send { key, msg } => {
                        eprintln!("received send for key {key}, message:{msg}");
                        let key_set = self.log.entry(key).or_insert(HashSet::new());
                        key_set.insert((self.count, msg));
                        eprintln!(" sending sendok with count {}", &self.count);
                        eprintln!("current log {:?}", &self.log);
                        response.body.payload = Payload::SendOk { offset: self.count };
                        response
                            .send_self(&mut *output)
                            .context("failed to respond to send request in replicated log")?;

                        self.count += 1;
                    }

                    Payload::Poll { offsets } => {
                        eprintln!("received a poll with dictionary:{:?}", &offsets);
                        eprintln!("current log:{:?}", &self.log);
                        // let mut ret_map: HashMap<String, HashSet<(usize, usize)>> = HashMap::new();
                        let mut ret_map: HashMap<String, Vec<(usize, usize)>> = HashMap::new();
                        for (k, v) in offsets {
                            eprintln!("IN POLL, offset loop, current key:{k}, value:{v}");

                            let Some(key_set) = self.log.get(&k) else {
                                eprintln!("KEY: {k} NOT FOUND in {:?}", self.log);
                                continue;
                            };
                            eprintln!("in POLL, key set before:{:?}", &key_set);
                            let mut     ret_set: Vec<(usize, usize)> = key_set
                                .clone() //remove clone
                                .into_iter()
                                .filter(|(log_key, _)| *log_key >= v)
                                .collect();
                            // let ret_set: Vec<(usize, usize)> = ret_set.s;
                            ret_set.sort();
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
                            // if !self.committed_offsets.contains_key(&key) {continue;}
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
    main_loop::<_, LogNode, _, _>(())
}

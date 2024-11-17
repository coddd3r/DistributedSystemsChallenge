use ds_challenge::*;

use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, ops::Neg};
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
    CommitOffset {
        offsets: HashMap<String, usize>,
    },
    CommitOffsetOk,
    ListCommittedOffsets {
        keys: Vec<String>,
    },
    ListCommittedOffsetsOk {
        offsets: HashMap<String, Option<usize>>,
    },
}

struct LogNode {
    node: String,
    id: usize,
    log: HashMap<String, Vec<usize>>,
}

impl Node<(), Payload> for LogNode {
    fn from_init(
        _state: (),
        init: Init,
        _inject: std::sync::mpsc::Sender<Event<Payload, ()>>,
    ) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            node: init.node_id,
            id: 0,
            log: HashMap::new(),
        })
    }
        
    fn handle_input(
        &mut self,
        input: Event<Payload, ()>,
        output: &mut std::io::StdoutLock,
    ) -> anyhow::Result<()> {
    }
}
fn main() -> anyhow::Result<()> {
    main_loop::<_, LogNode, _, _>(())
}

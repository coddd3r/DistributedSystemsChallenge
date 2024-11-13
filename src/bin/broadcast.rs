use ds_challenge::*;
use std::{
    collections::HashMap,
    io::{StdoutLock, Write},
};
// use ulid::Ulid;

use anyhow::Context;
use serde::{Deserialize, Serialize};

//basic skeleton of a network message
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Message<Payload> {
    src: String,
    dest: String,
    body: Body<Payload>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Body<Payload> {
    #[serde(rename = "msg_id")]
    id: Option<usize>,
    in_reply_to: Option<usize>,

    //type of message
    #[serde(flatten)]
    payload: Payload,
}

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
        messages: Vec<usize>,
    },
    Topology {
        topology: HashMap<String, Vec<String>>,
    },
    TopologyOk,
}

//node representing unique id
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BroadcastNode {
    node: String,
    id: usize,
    messages: Vec<usize>,
}

//handle basic Generate responses
impl Node<(), Payload> for BroadcastNode {
    fn from_init(_state: (), init: Init) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            node: init.node_id,
            id: 1,
            messages: Vec::new(),
        })
    }
    fn handle_input(
        &mut self,
        input: ds_challenge::Message<Payload>,
        output: &mut StdoutLock,
    ) -> anyhow::Result<()> {
        //derive a message response with the prev held payload
        let mut response = input.derive_response(Some(&mut self.id));
        //for each response type set payload to enum
        match response.body.payload {
            Payload::Broadcast { message } => {
                self.messages.push(message);
                response.body.payload = Payload::BroadcastOk;
                serde_json::to_writer(&mut *output, &response)
                    .context("error serializing broadcast reply")?;
                output.write_all(b"\n").context("write trailing newline")?;
                self.id += 1;
            }
            Payload::BroadcastOk { .. } => todo!(),

            Payload::Read => {
                response.body.payload = Payload::ReadOk {
                    messages: self.messages.clone(),
                };
                serde_json::to_writer(&mut *output, &response)
                    .context("error serializing broadcast reply")?;
                output.write_all(b"\n").context("write trailing newline")?;
                self.id += 1;
            }
            Payload::ReadOk { .. } => {}

            Payload::Topology { .. } => {
                response.body.payload = Payload::TopologyOk;
                serde_json::to_writer(&mut *output, &response)
                    .context("error serializing broadcast reply")?;
                output.write_all(b"\n").context("write trailing newline")?;
                self.id += 1;
            }
            Payload::TopologyOk {} => {}
        }

        Ok(())
    }
}
fn main() -> anyhow::Result<()> {
    //'_' represent unused state and payload generics for <S, N, P>
    main_loop::<_, BroadcastNode, _>(())
}

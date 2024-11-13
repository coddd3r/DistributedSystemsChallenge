use ds_challenge::*;
use std::io::{StdoutLock, Write};

use anyhow::Context;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]

//basic skeleton of a network message
struct Message<Payload> {
    src: String,
    dest: String,
    body: Body<Payload>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Body<Payload> {
    //message may have an id or not
    #[serde(rename = "msg_id")]
    id: Option<usize>,
    in_reply_to: Option<usize>,

    //type of message
    #[serde(flatten)]
    payload: Payload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]

//what type of message
enum Payload {
    Echo { echo: String },
    EchoOk { echo: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EchoNode {
    id: usize,
}

//handle basic echo responses
impl Node<(), Payload> for EchoNode {
    fn from_init(_state: (), _init: ds_challenge::Init) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(EchoNode { id: 1 })
    }
    fn handle_input(
        &mut self,
        input: ds_challenge::Message<Payload>,
        output: &mut StdoutLock,
    ) -> anyhow::Result<()> {
        match input.body.payload {
            //respond to a client
            Payload::Echo { echo } => {
                let reply = Message {
                    src: input.dest,
                    dest: input.src,
                    body: Body {
                        id: Some(self.id),
                        in_reply_to: input.body.id,
                        payload: Payload::EchoOk { echo },
                    },
                };
                serde_json::to_writer(&mut *output, &reply)?;
                output.write_all(b"\n").context("write trailing newline")?;
            }
            Payload::EchoOk { .. } => {}
        }

        Ok(())
    }
}
fn main() -> anyhow::Result<()> {
    //'_' represent unused state and payload generics
    main_loop::<_, EchoNode, _>(())
}

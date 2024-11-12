use ds_challenge::*;
use std::io::{StdoutLock, Write};

use anyhow::{bail, Context};
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
    // #[serde(rename = "type")]
    // ty: String,
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
    Echo {
        echo: String,
    },
    EchoOk {
        echo: String,
    },
    Init {
        node_id: String,
        node_ids: Vec<String>,
    },
    InitOk,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EchoNode {
    id: usize,
}

//handle basic echo responses
impl Node<Payload> for EchoNode {
    fn handle_input(
        &mut self,
        input: ds_challenge::Message<Payload>,
        output: &mut StdoutLock,
    ) -> anyhow::Result<()> {
        match input.body.payload {
            //initialization message
            Payload::Init { .. } => {
                let reply = Message {
                    src: input.dest,
                    dest: input.src,
                    body: Body {
                        id: Some(self.id),
                        in_reply_to: input.body.id,
                        payload: Payload::InitOk,
                    },
                };
                serde_json::to_writer(&mut *output, &reply)?;
                output.write_all(b"\n").context("write trailing newline")?;
            }
            Payload::InitOk { .. } => bail!("receievec init ok message"),
            //
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
    main_loop(EchoNode { id: 0 })
}

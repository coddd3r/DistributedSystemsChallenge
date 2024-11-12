use std::io::{StdoutLock, Write};

use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]

//basic skeleton of a network message
struct Message {
    src: String,
    dest: String,
    body: Body,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Body {
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

//what type fo message
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
impl EchoNode {
    pub fn handle_input(&mut self, input: Message, output: &mut StdoutLock) -> anyhow::Result<()> {
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
    //configure io with serde
    let stdin = std::io::stdin().lock();
    let inputs = serde_json::Deserializer::from_reader(stdin).into_iter::<Message>();

    let mut stdout = std::io::stdout().lock();
    // let mut output = serde_json::Serializer::new(stdout);

    //set up initial state
    let mut state = EchoNode { id: 0 };
    //handle every input read from STDIN
    for input in inputs {
        let input = input.context("input from stdin could  not be deserialized")?;
        state.handle_input(input, &mut stdout)?;
    }
    Ok(())
}

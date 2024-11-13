use ds_challenge::*;
use std::io::{StdoutLock, Write};
// use ulid::Ulid;

use anyhow::{bail, Context};
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
    Generate,
    GenerateOk {
        #[serde(rename = "id")]
        guid: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UniqueNode {
    node: String,
    id: usize,
}

//handle basic Generate responses
impl Node<(), Payload> for UniqueNode {
    fn from_init(_state: (), init: Init) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(UniqueNode {
            node: init.node_id,
            id: 1,
        })
    }
    fn handle_input(
        &mut self,
        input: ds_challenge::Message<Payload>,
        output: &mut StdoutLock,
    ) -> anyhow::Result<()> {
        match input.body.payload {
            //respond to a client
            Payload::Generate {} => {
                // let guid = Ulid::new().to_string();
                //{node-id}-{message_id} should always be unique
                //assumes nodes do not reuse node-ids on restart
                let guid = format!("{}-{}", self.node, self.id);
                let reply = Message {
                    src: input.dest,
                    dest: input.src,
                    body: Body {
                        id: Some(self.id),
                        in_reply_to: input.body.id,
                        payload: Payload::GenerateOk { guid },
                    },
                };
                serde_json::to_writer(&mut *output, &reply)?;
                output.write_all(b"\n").context("write trailing newline")?;
                self.id += 1;
            }
            Payload::GenerateOk { .. } => bail!("receievec generate ok message"),
        }

        Ok(())
    }
}
fn main() -> anyhow::Result<()> {
    //'_' represent unused state and payload generics for <S, N, P>
    main_loop::<_, UniqueNode, _>(())
}

use ds_challenge::*;
use std::io::StdoutLock;

use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};

//
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

//node representing unique id
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UniqueNode {
    node: String,
    id: usize,
}

//handle basic Generate responses
impl Node<(), Payload> for UniqueNode {
    fn from_init(
        _state: (),
        init: Init,
        _tx: std::sync::mpsc::Sender<Event<Payload>>,
    ) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            node: init.node_id,
            id: 1,
        })
    }

    fn handle_input(
        &mut self,
        input: ds_challenge::Event<Payload>,
        output: &mut StdoutLock,
    ) -> anyhow::Result<()> {
        let Event::Message(input) = input else {
            panic!("got unexpected injected event")
        };
        let mut response = input.derive_response(Some(&mut self.id));

        match response.body.payload {
            //generate a unique id for a message
            Payload::Generate {} => {
                //{producer(node)-name/id}-{message_id} should always be unique
                //assumes nodes do not reuse node-ids on restart
                response.body.payload = Payload::GenerateOk {
                    guid: format!("{}-{}", self.node, self.id),
                };
                response
                    .send_self(&mut *output)
                    .context("respond to generate unique id message")?;
            }

            Payload::GenerateOk { .. } => bail!("receieved generate ok message"),
        }

        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    //'_' represent unused state and payload generics for <S, N, P>
    main_loop::<_, UniqueNode, _, _>(())
}

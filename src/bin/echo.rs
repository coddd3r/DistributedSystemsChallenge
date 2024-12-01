/* 
    echo challenge 1 
*/

use ds_challenge::*;
use std::io::StdoutLock;

use anyhow::Context;
use serde::{Deserialize, Serialize};

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
    fn from_init(
        _state: (),
        _init: ds_challenge::Init,
        _tx: std::sync::mpsc::Sender<Event<Payload>>,
    ) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(EchoNode { id: 1 })
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
            //respond to a client
            Payload::Echo { echo } => {
                response.body.payload = Payload::EchoOk { echo };
                response
                    .send_self(&mut *output)
                    .context("respond to echo message")?;
            }
            Payload::EchoOk { .. } => {}
        }

        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    //'_' represent unused state,payload and injected-payload generics
    main_loop::<_, EchoNode, _, _>(())
}

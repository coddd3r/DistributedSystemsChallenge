use std::str;

use anyhow::Context;
use ds_challenge::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum Payload {
    Add { delta: usize },
    AddOk,
    Read,
    ReadOk { value: usize },
}

struct CounterNode {
    node: String,
    id: usize,
    delta: usize,
}

impl Node<(), Payload> for CounterNode {
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
            id: 1,
            delta: 0,
        })
    }

    fn handle_input(
        &mut self,
        input: Event<Payload, ()>,
        output: &mut std::io::StdoutLock,
    ) -> anyhow::Result<()> {
        let Event::Message(input) = input else {
            panic!("got unexpected injected event")
        };
        let mut response = input.derive_response(Some(&mut self.id));
        match response.body.payload {
            Payload::Add { delta } => {
                response.body.payload = Payload::AddOk;
                self.delta += delta;
                response
                    .send_self(&mut *output)
                    .context("add delta failure")?
            }
            Payload::AddOk => {}
            Payload::ReadOk { value } => {}
            Payload::Read => {
                response.body.payload = Payload::ReadOk { value: () }
            }
        }
        Ok(())
    }
}
fn main() -> anyhow::Result<()> {
    //'_' represent unused state, node, Payload and InjectedPayload generics for <S, N, P>
    main_loop::<_, CounterNode, _, _>(())
}

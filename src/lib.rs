use std::io::StdoutLock;

use anyhow::Context;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]

//basic skeleton of a network message
pub struct Message<Payload> {
    pub src: String,
    pub dest: String,
    pub body: Body<Payload>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Body<Payload> {
    // #[serde(rename = "type")]
    // ty: String,
    #[serde(rename = "msg_id")]
    pub id: Option<usize>,
    pub in_reply_to: Option<usize>,

    //type of message
    #[serde(flatten)]
    pub payload: Payload,
}

pub struct Init {
    pub node_id: String,
    pub node_ids: Vec<String>,
}

pub trait Node<Payload> {
    fn handle_input(
        &mut self,
        input: Message<Payload>,
        output: &mut StdoutLock,
    ) -> anyhow::Result<()>;
}

//S=State
//take in shared state and manipulate according to stdin input
pub fn main_loop<S, Payload>(mut state: S) -> anyhow::Result<()>
where
    S: Node<Payload>,
    Payload: DeserializeOwned,
{
    //configure io with serde
    let stdin = std::io::stdin().lock();
    let inputs = serde_json::Deserializer::from_reader(stdin).into_iter::<Message<Payload>>();

    let mut stdout = std::io::stdout().lock();
    // let mut output = serde_json::Serializer::new(stdout);

    //set up initial state
    // let mut state = EchoNode { id: 0 };
    //handle every input read from STDIN
    for input in inputs {
        let input = input.context("input from stdin could  not be deserialized")?;
        state.handle_input(input, &mut stdout)?;
    }
    Ok(())
}

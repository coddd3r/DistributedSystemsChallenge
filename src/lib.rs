use std::{
    fmt::Debug,
    io::{BufRead, StdoutLock, Write},
};

use anyhow::Context;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

//basic skeleton of a network message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message<Payload> {
    pub src: String,
    pub dest: String,
    pub body: Body<Payload>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Body<Payload> {
    #[serde(rename = "msg_id")]
    pub id: Option<usize>,
    pub in_reply_to: Option<usize>,

    //type of message
    #[serde(flatten)]
    pub payload: Payload,
}

//handle init message that sends a lsit of nodes and this current nodes id
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Init {
    pub node_id: String,
    pub node_ids: Vec<String>,
}

//init message types(received, ok-response)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum InitPayload {
    Init(Init),
    InitOk,
}

pub trait Node<S, Payload> {
    fn from_init(state: S, init: Init) -> anyhow::Result<Self>
    where
        Self: Sized;

    fn handle_input(
        &mut self,
        input: Message<Payload>,
        output: &mut StdoutLock,
    ) -> anyhow::Result<()>;
}

//Generics: S=State, N:Node, P:Payload
//take in shared state and manipulate according to stdin input
pub fn main_loop<S, N, P>(initial_state: S) -> anyhow::Result<()>
where
    P: DeserializeOwned + Debug,
    N: Node<S, P>,
{
    //configure io with serde
    let stdin = std::io::stdin().lock();
    let mut stdout = std::io::stdout().lock();
    let mut stdin = stdin.lines();

    let init_msg: Message<InitPayload> = serde_json::from_str(
        &stdin
            .next()
            .expect("no init message received")
            .context("failed to read init message")?,
    )
    .context("failed to deserialize input message")?;

    let InitPayload::Init(init) = init_msg.body.payload else {
        panic!("NO INIT MESSAGE RECEIVED");
    };

    let mut node: N = Node::from_init(initial_state, init).context("node initialization failed")?;

    let reply = Message {
        src: init_msg.dest,
        dest: init_msg.src,
        body: Body {
            //message id 0 reserved for init message
            id: Some(0),
            in_reply_to: init_msg.body.id,
            payload: InitPayload::InitOk,
        },
    };
    serde_json::to_writer(&mut stdout, &reply).context("serialize response to init")?;
    stdout.write_all(b"\n").context("write extra newline")?;

    //handle every input read from STDIN
    for line in stdin {
        let line = line.context("input from stdin could  not be read")?;
        let input: Message<P> =
            serde_json::from_str(&line).context("could not deserialize input line")?;
        node.handle_input(input, &mut stdout)
            .context("Node failed to handle input")?;
    }
    Ok(())
}

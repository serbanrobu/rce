use anyhow::{Context, Result};
use async_bincode::AsyncBincodeReader;
use futures::prelude::*;
use rce::{Frame, Request};
use std::net::SocketAddr;
use std::process;
use structopt::StructOpt;
use tokio::io::{self, AsyncWriteExt};
use tokio::net::TcpStream;

/// Remote Command Execution
#[derive(StructOpt, Debug)]
struct Opt {
    /// Remote Socket Address
    #[structopt(short, long, default_value = "127.0.0.1:8080")]
    addr: SocketAddr,
    /// List of remote command's environment variables
    #[structopt(short, long)]
    env: Vec<String>,
    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(StructOpt, Debug)]
enum Command {
    #[structopt(external_subcommand)]
    Other(Vec<String>),
}

#[tokio::main]
pub async fn main() -> Result<()> {
    let opt = Opt::from_args();

    let envs = opt
        .env
        .into_iter()
        .map(|e| match e.find('=') {
            Some(i) => (e[..i].to_owned(), e[i + 1..].to_owned()),
            _ => (e, "".to_owned()),
        })
        .collect();

    let Command::Other(args) = opt.cmd;
    let (cmd, args) = args
        .split_first()
        .map(|(c, a)| (c.to_owned(), a.to_owned()))
        .unwrap();

    let req = Request { cmd, args, envs };

    let mut socket = TcpStream::connect(&opt.addr).await?;
    socket.write(&bincode::serialize(&req)?).await?;
    socket.shutdown().await?;

    let mut stream = AsyncBincodeReader::from(socket);

    while let Some(frame) = stream.next().await {
        match frame? {
            Frame::Stdout(bytes) => io::stdout().write(&bytes).await?,
            Frame::Stderr(bytes) => io::stderr().write(&bytes).await?,
            Frame::Status(code) => process::exit(code.context("process interrupted")?),
        };
    }

    Ok(())
}

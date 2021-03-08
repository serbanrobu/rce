use anyhow::{Error, Result};
use async_bincode::AsyncBincodeWriter;
use futures::SinkExt;
use rce::{Frame, Request};
use std::net::SocketAddr;
use std::process::Stdio;
use structopt::StructOpt;
use tokio::io::{self, AsyncRead, AsyncReadExt};
use tokio::net::TcpListener;
use tokio::process::Command;
use tokio_stream::{Stream, StreamExt};
use tokio_util::codec::{BytesCodec, FramedRead};

/// Remote Command Execution Agent
#[derive(StructOpt, Debug)]
struct Opt {
    /// Socket Address
    #[structopt(short, long, default_value = "127.0.0.1:8080")]
    addr: SocketAddr,
}

fn into_frame_stream(
    reader: impl AsyncRead,
    op: fn(Vec<u8>) -> Frame,
) -> impl Stream<Item = io::Result<Frame>> {
    FramedRead::new(reader, BytesCodec::new()).map(move |r| r.map(|b| b.to_vec()).map(op))
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt = Opt::from_args();

    let listener = TcpListener::bind(&opt.addr).await?;
    println!("Listening on: {}", opt.addr);

    loop {
        let (mut socket, _) = listener.accept().await?;

        tokio::spawn(async move {
            let mut buf = Vec::new();
            socket.read_to_end(&mut buf).await?;
            let req: Request = bincode::deserialize(&buf)?;

            let mut child = Command::new(req.cmd)
                .args(&req.args)
                .envs(req.envs)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;

            let stdout = into_frame_stream(child.stdout.take().unwrap(), Frame::Stdout);
            let stderr = into_frame_stream(child.stderr.take().unwrap(), Frame::Stderr);
            let mut stream = stdout.merge(stderr);
            let mut sink = AsyncBincodeWriter::from(socket).for_async();

            while let Some(item) = stream.next().await {
                let frame = item?;
                sink.send(frame).await?;
            }

            let status = child.wait().await?;
            sink.send(Frame::Status(status.code())).await?;

            Ok::<_, Error>(())
        });
    }
}

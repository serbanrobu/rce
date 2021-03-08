use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum Frame {
    Stdout(Vec<u8>),
    Stderr(Vec<u8>),
    Status(Option<i32>),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Request {
    pub cmd: String,
    pub envs: Vec<(String, String)>,
    pub args: Vec<String>,
}

#[macro_use]
extern crate tracing;

mod rpc;
mod storage;
mod client;

use std::sync::Arc;

use openraft::{Raft, BasicNode, Config};
use serde::{Deserialize, Serialize};
pub use storage::{
    FromRow,
    RaftStore,
    Request,
    Response,
    SqliteMemory,
    StateMachine,
    StorageHandle,
};

use crate::rpc::ReplicatNetwork;

pub use client::ReplicatClient;

pub type NodeId = u64;
pub type ReplicatRaft = Raft<TypeConfig, Arc<ReplicatNetwork>, Arc<RaftStore>>;

openraft::declare_raft_types!(
    pub TypeConfig: D = Request, R = Response, NodeId = NodeId, Node = BasicNode
);

pub(crate) fn to_bytes<T: Serialize>(v: &T) -> Result<Vec<u8>, rmp_serde::encode::Error> {
    rmp_serde::to_vec(v)
}

pub(crate) fn from_bytes<'a, T: Deserialize<'a>>(
    buf: &'a [u8],
) -> Result<T, rmp_serde::decode::Error> {
    rmp_serde::from_slice(buf)
}


pub async fn start_node() -> anyhow::Result<()> {

    Ok(())
}
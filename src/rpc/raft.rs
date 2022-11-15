use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use openraft::error::{
    AppendEntriesError,
    InstallSnapshotError,
    NetworkError,
    RPCError,
    VoteError,
};
use openraft::raft::{
    AppendEntriesRequest,
    AppendEntriesResponse,
    InstallSnapshotRequest,
    InstallSnapshotResponse,
    VoteRequest,
    VoteResponse,
};
use openraft::{BasicNode, RaftNetwork, RaftNetworkFactory};
use parking_lot::RwLock;

use crate::rpc::client::RpcClient;
use crate::{NodeId, TypeConfig};

pub struct ReplicatNetwork {
    clients: RwLock<HashMap<NodeId, (Cow<'static, str>, RpcClient)>>,
}

impl ReplicatNetwork {
    /// Attempts to get an existing handle to the RPC client, or creates a new client.
    pub(crate) fn get_or_connect_client(
        &self,
        node_id: NodeId,
        addr: &str,
    ) -> Result<RpcClient, NetworkError> {
        let existing = { self.clients.read().get(&node_id).cloned() };

        if let Some((addr, client)) = existing {
            if addr == addr {
                return Ok(client);
            }
        }

        let addr = Cow::Owned(addr.to_string());
        let client = RpcClient::connect(&addr).map_err(|e| NetworkError::new(&e))?;

        {
            self.clients
                .write()
                .insert(node_id, (addr.clone(), client.clone()));
        }

        info!(remote_node_id = node_id, remote_addr = %addr, "Connected to remote node.");

        Ok(client)
    }
}

#[async_trait::async_trait]
impl RaftNetworkFactory<TypeConfig> for Arc<ReplicatNetwork> {
    type Network = NetworkConnection;
    type ConnectionError = NetworkError;

    async fn new_client(
        &mut self,
        node_id: NodeId,
        node: &BasicNode,
    ) -> Result<Self::Network, Self::ConnectionError> {
        let client = self.get_or_connect_client(node_id, node.addr.as_str())?;
        Ok(NetworkConnection { node_id, client })
    }
}

pub struct NetworkConnection {
    node_id: NodeId,
    client: RpcClient,
}

#[async_trait::async_trait]
impl RaftNetwork<TypeConfig> for NetworkConnection {
    async fn send_append_entries(
        &mut self,
        rpc: AppendEntriesRequest<TypeConfig>,
    ) -> Result<
        AppendEntriesResponse<NodeId>,
        RPCError<NodeId, BasicNode, AppendEntriesError<NodeId>>,
    > {
        self.client
            .append_entries(rpc)
            .await
            .map_err(|e| {
                warn!(error = ?e, remote_node_id = self.node_id, "Unable to send append entries due to error.");
                RPCError::Network(NetworkError::new(&e))
            })
    }

    async fn send_install_snapshot(
        &mut self,
        rpc: InstallSnapshotRequest<TypeConfig>,
    ) -> Result<
        InstallSnapshotResponse<NodeId>,
        RPCError<NodeId, BasicNode, InstallSnapshotError<NodeId>>,
    > {
        self.client
            .install_snapshot(rpc)
            .await
            .map_err(|e| {
                warn!(error = ?e, remote_node_id = self.node_id, "Unable to send append entries due to error.");
                RPCError::Network(NetworkError::new(&e))
            })
    }

    async fn send_vote(
        &mut self,
        rpc: VoteRequest<NodeId>,
    ) -> Result<VoteResponse<NodeId>, RPCError<NodeId, BasicNode, VoteError<NodeId>>>
    {
        self.client
            .vote(rpc)
            .await
            .map_err(|e| {
                warn!(error = ?e, remote_node_id = self.node_id, "Unable to send append entries due to error.");
                RPCError::Network(NetworkError::new(&e))
            })
    }
}

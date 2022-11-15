use openraft::{BasicNode, raft, RaftMetrics};
use tonic::codegen::http::uri::InvalidUri;
use tonic::transport::{Channel, Uri};

use crate::rpc::rpc_models::cluster_rpc_client::ClusterRpcClient;
use crate::rpc::rpc_models::{AppendEntriesRequest, ChangeMembershipRequest, InstallSnapshotRequest, LogId, MetricsRequest, MutateRequest, Node, RpcError, Vote, VoteRequest};
use crate::{TypeConfig, to_bytes, from_bytes, NodeId};
use crate::storage::params::TransportableParam;


pub(crate) struct RpcClient {
    channel: Channel,
    client: ClusterRpcClient<Channel>,
}

impl RpcClient {
    pub(crate) fn connect(addr: &str) -> Result<Self, InvalidUri> {
        let uri = format!("http://{}", addr).parse::<Uri>()?;

        let endpoint = Channel::builder(uri);
        let channel = endpoint.connect_lazy();

        let client = ClusterRpcClient::new(channel.clone());

        Ok(Self { channel, client })
    }

    pub(crate) async fn append_entries(&mut self, req: raft::AppendEntriesRequest<TypeConfig>) -> Result<raft::AppendEntriesResponse<NodeId>, Error> {
        let entries_byes = to_bytes(&req.entries)?;

        let req = AppendEntriesRequest {
            vote: Some(Vote {
                term: req.vote.term,
                node_id: req.vote.node_id,
                committed: req.vote.committed,
            }),
            prev_log_id: req.prev_log_id.map(|log_id| LogId {
                term: log_id.leader_id.term,
                node_id: log_id.leader_id.node_id,
                index: log_id.index
            }),
            entries: entries_byes,
            leader_commit: req.leader_commit.map(|log_id| LogId {
                term: log_id.leader_id.term,
                node_id: log_id.leader_id.node_id,
                index: log_id.index
            }),
        };

        let resp = self.client
            .append_entries(req)
            .await?
            .into_inner();

        let resp = from_bytes(&resp.additional_data)?;

        Ok(resp)
    }

    pub(crate) async fn install_snapshot(&mut self, req: raft::InstallSnapshotRequest<TypeConfig>) -> Result<raft::InstallSnapshotResponse<NodeId>, Error> {
        let metadata = to_bytes(&req.meta)?;

        let req = InstallSnapshotRequest {
            vote: Some(Vote {
                term: req.vote.term,
                node_id: req.vote.node_id,
                committed: req.vote.committed,
            }),
            meta: metadata,
            offset: req.offset,
            data: req.data,
            done: req.done
        };

        let resp = self.client
            .install_snapshot(req)
            .await?
            .into_inner();

        let resp = from_bytes(&resp.additional_data)?;

        Ok(resp)
    }

    pub(crate) async fn vote(&mut self, req: raft::VoteRequest<NodeId>) -> Result<raft::VoteResponse<NodeId>, Error> {
        let req = VoteRequest {
            vote: Some(Vote {
                term: req.vote.term,
                node_id: req.vote.node_id,
                committed: req.vote.committed,
            }),
            last_log_id: req.last_log_id.map(|log_id| LogId {
                term: log_id.leader_id.term,
                node_id: log_id.leader_id.node_id,
                index: log_id.index
            }),
        };

        let resp = self.client
            .vote(req)
            .await?
            .into_inner();

        let resp = from_bytes(&resp.additional_data)?;

        Ok(resp)
    }

    pub(crate) async fn add_learner(&mut self, node_id: NodeId, addr: String) -> Result<raft::AddLearnerResponse<NodeId>, Error> {
        let req = Node { id: node_id, rpc_addr: addr };

        let resp = self.client
            .add_learner(req)
            .await?
            .into_inner();

        let resp = from_bytes(&resp.additional_data)?;

        Ok(resp)
    }

    pub(crate) async fn change_membership(&mut self, members: Vec<NodeId>) -> Result<(), Error> {
        let req = ChangeMembershipRequest { members };

        let _resp = self.client
            .change_membership(req)
            .await?
            .into_inner();

        Ok(())
    }

    pub(crate) async fn get_metrics(&mut self) -> Result<RaftMetrics<NodeId, BasicNode>, Error> {
        let resp = self.client
            .get_metrics(MetricsRequest {})
            .await?
            .into_inner();

        let resp = from_bytes(&resp.additional_data)?;

        Ok(resp)
    }

    pub(crate) async fn mutate_state(&mut self, sql: String, params: &[TransportableParam]) -> Result<(), Error> {
        let parameters = to_bytes(&params)?;

        let req = MutateRequest { sql, parameters };

        let resp = self.client
            .mutate_state(req)
            .await?
            .into_inner();

        if resp.error == RpcError::ForwardToLeader as i32 {
            return Err(Error::UpdateLeader(resp.leader.unwrap()))
        }

        Ok(())
    }
}

impl Clone for RpcClient {
    fn clone(&self) -> Self {
        Self {
            client: ClusterRpcClient::new(self.channel.clone()),
            channel: self.channel.clone(),
        }
    }
}


#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    NetworkError(#[from] tonic::Status),

    #[error("{0}")]
    SerializationError(#[from] rmp_serde::encode::Error),

    #[error("{0}")]
    DeserializationError(#[from] rmp_serde::decode::Error),

    #[error("Leader must be updated to {0:?}")]
    UpdateLeader(Node),
    
    #[error("Node was unable to contact a viable leader.")]
    UnknownLeader,
}
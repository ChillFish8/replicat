use std::ops::Deref;
use std::sync::Arc;
use openraft::{BasicNode, raft};
use openraft::error::{AddLearnerError, AppendEntriesError, ChangeMembershipError, ClientWriteError, Fatal, ForwardToLeader, InstallSnapshotError, NetworkError, VoteError};
use serde::Serialize;
use tonic::{Code, Request, Response, Status};

use crate::rpc::rpc_models::cluster_rpc_server::ClusterRpc;
use crate::rpc::rpc_models::{AppendEntriesRequest, ChangeMembershipRequest, InstallSnapshotRequest, LogId, MetricsRequest, MutateRequest, Node, RpcError, RpcResponse, Vote, VoteRequest};
use crate::{from_bytes, NodeId, ReplicatRaft, to_bytes};

pub struct RpcServer {
    raft: Arc<ReplicatRaft>,
}

#[async_trait::async_trait]
impl ClusterRpc for RpcServer {
    async fn append_entries(
        &self,
        request: Request<AppendEntriesRequest>,
    ) -> Result<Response<RpcResponse>, Status> {
        let request = request.into_inner();
        let vote = request.vote.unwrap();
        let entries = from_bytes(&request.entries)
            .map_err(|e| Status::new(Code::Internal, e.to_string()))?;

        let rpc = raft::AppendEntriesRequest {
            vote: vote.into(),
            prev_log_id: request.prev_log_id.map(openraft::LogId::from),
            entries,
            leader_commit: request.leader_commit.map(openraft::LogId::from),
        };

        let res = self.raft
            .append_entries(rpc)
            .await;

        let resp = match res {
            Ok(res) => RpcResponse::ok(res)?,
            Err(e) => {
                let AppendEntriesError::Fatal(error) = e;
                fatal_resp(error)?
            }
        };

        Ok(Response::new(resp))
    }

    async fn install_snapshot(
        &self,
        request: Request<InstallSnapshotRequest>,
    ) -> Result<Response<RpcResponse>, Status> {
        let request = request.into_inner();
        let vote = request.vote.unwrap();
        let metadata = from_bytes(&request.meta)
            .map_err(|e| Status::new(Code::Internal, e.to_string()))?;

        let rpc = raft::InstallSnapshotRequest {
            vote: vote.into(),
            meta: metadata,
            offset: request.offset,
            data: request.data,
            done: request.done,
        };

        let res = self.raft
            .install_snapshot(rpc)
            .await;

        let resp = match res {
            Ok(res) => RpcResponse::ok(res)?,
            Err(e) => {
                match e {
                    InstallSnapshotError::SnapshotMismatch(error) => {
                         let data = to_bytes(&error)
                             .map_err(|e| Status::new(Code::Internal, e.to_string()))?;

                         RpcResponse {
                             error: RpcError::SnapshotMismatch as _,
                             leader: None,
                             message: "The provided snapshot does not match.".to_string(),
                             additional_data: data,
                         }
                    }
                    InstallSnapshotError::Fatal(error) => fatal_resp(error)?,
                }
            }
        };

        Ok(Response::new(resp))
    }

    async fn vote(
        &self,
        request: Request<VoteRequest>,
    ) -> Result<Response<RpcResponse>, Status> {
        let request = request.into_inner();
        let vote = request.vote.unwrap();

        let rpc = raft::VoteRequest {
            vote: vote.into(),
            last_log_id: request.last_log_id.map(openraft::LogId::from),
        };

        let res = self.raft
            .vote(rpc)
            .await;

        let resp = match res {
            Ok(res) => RpcResponse::ok(res)?,
            Err(e) => {
                let VoteError::Fatal(error) = e;
                fatal_resp(error)?
            }
        };

        Ok(Response::new(resp))
    }

    async fn add_learner(
        &self,
        request: Request<Node>,
    ) -> Result<Response<RpcResponse>, Status> {
        let node = request.into_inner();

        let basic_node = BasicNode::new(node.rpc_addr);

        let res = self.raft
            .add_learner(node.id, basic_node, false)
            .await;

        let resp = match res {
            Ok(res) => RpcResponse::ok(res)?,
            Err(e) => {
                 match e {
                     AddLearnerError::ForwardToLeader(leader) =>
                        forward_to_leader_resp(leader)?,
                     AddLearnerError::NetworkError(error) =>
                        network_error_resp(error)?,
                     AddLearnerError::Fatal(error) =>
                        fatal_resp(error)?,
                 }
            }
        };

        Ok(Response::new(resp))
    }

    async fn change_membership(
        &self,
        request: Request<ChangeMembershipRequest>,
    ) -> Result<Response<RpcResponse>, Status> {
        let request = request.into_inner();

        let res = self.raft
            .change_membership(
                request.members,
                true,
                true,
            )
            .await;

        let resp = match res {
            Ok(res) => RpcResponse::ok(res)?,
            Err(e) => {
                 match e {
                     ClientWriteError::ForwardToLeader(leader) =>
                         forward_to_leader_resp(leader)?,
                     ClientWriteError::ChangeMembershipError(error) =>
                         change_membership_resp(error)?,
                     ClientWriteError::Fatal(error) =>
                         fatal_resp(error)?,
                 }
            }
        };

        Ok(Response::new(resp))
    }

    async fn get_metrics(
        &self,
        _request: Request<MetricsRequest>,
    ) -> Result<Response<RpcResponse>, Status> {
        let metrics = self.raft.metrics();
        let metrics_ref = metrics.borrow();

        Ok(Response::new(RpcResponse::ok(metrics_ref.deref())?))
    }

    async fn mutate_state(
        &self,
        request: Request<MutateRequest>,
    ) -> Result<Response<RpcResponse>, Status> {
        let request = request.into_inner();
        todo!()
    }
}


impl RpcResponse {
    fn ok(v: impl Serialize) -> Result<Self, Status> {
        let data = to_bytes(&v)
            .map_err(|e| Status::new(Code::Internal, e.to_string()))?;

        Ok(RpcResponse {
            error: RpcError::None as _,
            leader: None,
            message: "Request OK".to_string(),
            additional_data: data
        })
    }
}

impl From<Vote> for openraft::Vote<NodeId> {
    fn from(vote: Vote) -> Self {
        openraft::Vote {
            term: vote.term,
            node_id: vote.node_id,
            committed: vote.committed,
        }
    }
}

impl From<LogId> for openraft::LogId<NodeId> {
    fn from(log_id: LogId) -> Self {
        openraft::LogId {
            leader_id: openraft::LeaderId {
                term: log_id.term,
                node_id: log_id.node_id,
            },
            index: log_id.index,
        }
    }
}

fn forward_to_leader_resp(leader: ForwardToLeader<NodeId, BasicNode>) -> Result<RpcResponse, Status> {
    let leader = leader.leader_id
         .and_then(|id| Some((id, leader.leader_node?)))
         .map(|(id, node)| Node { id, rpc_addr: node.addr });

    Ok(RpcResponse {
        error: RpcError::ForwardToLeader as _,
        leader,
        message: "Forward request to leader node.".to_string(),
        additional_data: vec![],
    })
}

fn change_membership_resp(error: ChangeMembershipError<NodeId>) -> Result<RpcResponse, Status> {
     let data = to_bytes(&error)
         .map_err(|e| Status::new(Code::Internal, e.to_string()))?;

     Ok(RpcResponse {
         error: RpcError::ChangeMembership as _,
         leader: None,
         message: "Change the Raft membership.".to_string(),
         additional_data: data,
     })
}

fn network_error_resp(error: NetworkError) -> Result<RpcResponse, Status> {
     let data = to_bytes(&error)
         .map_err(|e| Status::new(Code::Internal, e.to_string()))?;

     Ok(RpcResponse {
         error: RpcError::Network as _,
         leader: None,
         message: "A network error has occurred.".to_string(),
         additional_data: data,
     })
}

fn fatal_resp(error: Fatal<NodeId>) -> Result<RpcResponse, Status> {
     let data = to_bytes(&error)
         .map_err(|e| Status::new(Code::Internal, e.to_string()))?;

     Ok(RpcResponse {
         error: RpcError::Fatal as _,
         leader: None,
         message: "Fatal error has occurred.".to_string(),
         additional_data: data,
     })
}
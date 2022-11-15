#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Node {
    #[prost(uint64, tag = "1")]
    pub id: u64,
    #[prost(string, tag = "2")]
    pub rpc_addr: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RpcResponse {
    #[prost(enumeration = "RpcError", tag = "1")]
    pub error: i32,
    #[prost(message, optional, tag = "2")]
    pub leader: ::core::option::Option<Node>,
    #[prost(string, tag = "3")]
    pub message: ::prost::alloc::string::String,
    #[prost(bytes = "vec", tag = "4")]
    pub additional_data: ::prost::alloc::vec::Vec<u8>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AppendEntriesRequest {
    #[prost(message, optional, tag = "1")]
    pub vote: ::core::option::Option<Vote>,
    #[prost(message, optional, tag = "2")]
    pub prev_log_id: ::core::option::Option<LogId>,
    #[prost(bytes = "vec", tag = "3")]
    pub entries: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, optional, tag = "4")]
    pub leader_commit: ::core::option::Option<LogId>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InstallSnapshotRequest {
    #[prost(message, optional, tag = "1")]
    pub vote: ::core::option::Option<Vote>,
    #[prost(bytes = "vec", tag = "2")]
    pub meta: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint64, tag = "3")]
    pub offset: u64,
    #[prost(bytes = "vec", tag = "4")]
    pub data: ::prost::alloc::vec::Vec<u8>,
    #[prost(bool, tag = "5")]
    pub done: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct VoteRequest {
    #[prost(message, optional, tag = "1")]
    pub vote: ::core::option::Option<Vote>,
    #[prost(message, optional, tag = "2")]
    pub last_log_id: ::core::option::Option<LogId>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ChangeMembershipRequest {
    #[prost(uint64, repeated, tag = "1")]
    pub members: ::prost::alloc::vec::Vec<u64>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MetricsRequest {}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MutateRequest {
    #[prost(string, tag = "1")]
    pub sql: ::prost::alloc::string::String,
    #[prost(bytes = "vec", tag = "2")]
    pub parameters: ::prost::alloc::vec::Vec<u8>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Vote {
    #[prost(uint64, tag = "1")]
    pub term: u64,
    #[prost(uint64, tag = "2")]
    pub node_id: u64,
    #[prost(bool, tag = "3")]
    pub committed: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LogId {
    #[prost(uint64, tag = "1")]
    pub term: u64,
    #[prost(uint64, tag = "2")]
    pub node_id: u64,
    #[prost(uint64, tag = "3")]
    pub index: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum RpcError {
    None = 0,
    ForwardToLeader = 1,
    ChangeMembership = 2,
    Fatal = 3,
    Network = 4,
    SnapshotMismatch = 5,
}
impl RpcError {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            RpcError::None => "None",
            RpcError::ForwardToLeader => "ForwardToLeader",
            RpcError::ChangeMembership => "ChangeMembership",
            RpcError::Fatal => "Fatal",
            RpcError::Network => "Network",
            RpcError::SnapshotMismatch => "SnapshotMismatch",
        }
    }
}
/// Generated client implementations.
pub mod cluster_rpc_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    #[derive(Debug, Clone)]
    pub struct ClusterRpcClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl ClusterRpcClient<tonic::transport::Channel> {
        /// Attempt to create a new client by connecting to a given endpoint.
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: std::convert::TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> ClusterRpcClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_origin(inner: T, origin: Uri) -> Self {
            let inner = tonic::client::Grpc::with_origin(inner, origin);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> ClusterRpcClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T::ResponseBody: Default,
            T: tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
            >>::Error: Into<StdError> + Send + Sync,
        {
            ClusterRpcClient::new(InterceptedService::new(inner, interceptor))
        }
        /// Compress requests with the given encoding.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// error.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.send_compressed(encoding);
            self
        }
        /// Enable decompressing responses.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.accept_compressed(encoding);
            self
        }
        pub async fn append_entries(
            &mut self,
            request: impl tonic::IntoRequest<super::AppendEntriesRequest>,
        ) -> Result<tonic::Response<super::RpcResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/rpc_models.ClusterRpc/append_entries",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn install_snapshot(
            &mut self,
            request: impl tonic::IntoRequest<super::InstallSnapshotRequest>,
        ) -> Result<tonic::Response<super::RpcResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/rpc_models.ClusterRpc/install_snapshot",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn vote(
            &mut self,
            request: impl tonic::IntoRequest<super::VoteRequest>,
        ) -> Result<tonic::Response<super::RpcResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/rpc_models.ClusterRpc/vote",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn add_learner(
            &mut self,
            request: impl tonic::IntoRequest<super::Node>,
        ) -> Result<tonic::Response<super::RpcResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/rpc_models.ClusterRpc/add_learner",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn change_membership(
            &mut self,
            request: impl tonic::IntoRequest<super::ChangeMembershipRequest>,
        ) -> Result<tonic::Response<super::RpcResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/rpc_models.ClusterRpc/change_membership",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn get_metrics(
            &mut self,
            request: impl tonic::IntoRequest<super::MetricsRequest>,
        ) -> Result<tonic::Response<super::RpcResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/rpc_models.ClusterRpc/get_metrics",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn mutate_state(
            &mut self,
            request: impl tonic::IntoRequest<super::MutateRequest>,
        ) -> Result<tonic::Response<super::RpcResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/rpc_models.ClusterRpc/mutate_state",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
    }
}
/// Generated server implementations.
pub mod cluster_rpc_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    ///Generated trait containing gRPC methods that should be implemented for use with ClusterRpcServer.
    #[async_trait]
    pub trait ClusterRpc: Send + Sync + 'static {
        async fn append_entries(
            &self,
            request: tonic::Request<super::AppendEntriesRequest>,
        ) -> Result<tonic::Response<super::RpcResponse>, tonic::Status>;
        async fn install_snapshot(
            &self,
            request: tonic::Request<super::InstallSnapshotRequest>,
        ) -> Result<tonic::Response<super::RpcResponse>, tonic::Status>;
        async fn vote(
            &self,
            request: tonic::Request<super::VoteRequest>,
        ) -> Result<tonic::Response<super::RpcResponse>, tonic::Status>;
        async fn add_learner(
            &self,
            request: tonic::Request<super::Node>,
        ) -> Result<tonic::Response<super::RpcResponse>, tonic::Status>;
        async fn change_membership(
            &self,
            request: tonic::Request<super::ChangeMembershipRequest>,
        ) -> Result<tonic::Response<super::RpcResponse>, tonic::Status>;
        async fn get_metrics(
            &self,
            request: tonic::Request<super::MetricsRequest>,
        ) -> Result<tonic::Response<super::RpcResponse>, tonic::Status>;
        async fn mutate_state(
            &self,
            request: tonic::Request<super::MutateRequest>,
        ) -> Result<tonic::Response<super::RpcResponse>, tonic::Status>;
    }
    #[derive(Debug)]
    pub struct ClusterRpcServer<T: ClusterRpc> {
        inner: _Inner<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
    }
    struct _Inner<T>(Arc<T>);
    impl<T: ClusterRpc> ClusterRpcServer<T> {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }
        pub fn from_arc(inner: Arc<T>) -> Self {
            let inner = _Inner(inner);
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
            }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
        /// Enable decompressing requests with the given encoding.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.accept_compression_encodings.enable(encoding);
            self
        }
        /// Compress responses with the given encoding, if the client supports it.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.send_compression_encodings.enable(encoding);
            self
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for ClusterRpcServer<T>
    where
        T: ClusterRpc,
        B: Body + Send + 'static,
        B::Error: Into<StdError> + Send + 'static,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(
            &mut self,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/rpc_models.ClusterRpc/append_entries" => {
                    #[allow(non_camel_case_types)]
                    struct append_entriesSvc<T: ClusterRpc>(pub Arc<T>);
                    impl<
                        T: ClusterRpc,
                    > tonic::server::UnaryService<super::AppendEntriesRequest>
                    for append_entriesSvc<T> {
                        type Response = super::RpcResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::AppendEntriesRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).append_entries(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = append_entriesSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/rpc_models.ClusterRpc/install_snapshot" => {
                    #[allow(non_camel_case_types)]
                    struct install_snapshotSvc<T: ClusterRpc>(pub Arc<T>);
                    impl<
                        T: ClusterRpc,
                    > tonic::server::UnaryService<super::InstallSnapshotRequest>
                    for install_snapshotSvc<T> {
                        type Response = super::RpcResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::InstallSnapshotRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).install_snapshot(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = install_snapshotSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/rpc_models.ClusterRpc/vote" => {
                    #[allow(non_camel_case_types)]
                    struct voteSvc<T: ClusterRpc>(pub Arc<T>);
                    impl<T: ClusterRpc> tonic::server::UnaryService<super::VoteRequest>
                    for voteSvc<T> {
                        type Response = super::RpcResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::VoteRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).vote(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = voteSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/rpc_models.ClusterRpc/add_learner" => {
                    #[allow(non_camel_case_types)]
                    struct add_learnerSvc<T: ClusterRpc>(pub Arc<T>);
                    impl<T: ClusterRpc> tonic::server::UnaryService<super::Node>
                    for add_learnerSvc<T> {
                        type Response = super::RpcResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::Node>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).add_learner(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = add_learnerSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/rpc_models.ClusterRpc/change_membership" => {
                    #[allow(non_camel_case_types)]
                    struct change_membershipSvc<T: ClusterRpc>(pub Arc<T>);
                    impl<
                        T: ClusterRpc,
                    > tonic::server::UnaryService<super::ChangeMembershipRequest>
                    for change_membershipSvc<T> {
                        type Response = super::RpcResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ChangeMembershipRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).change_membership(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = change_membershipSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/rpc_models.ClusterRpc/get_metrics" => {
                    #[allow(non_camel_case_types)]
                    struct get_metricsSvc<T: ClusterRpc>(pub Arc<T>);
                    impl<
                        T: ClusterRpc,
                    > tonic::server::UnaryService<super::MetricsRequest>
                    for get_metricsSvc<T> {
                        type Response = super::RpcResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::MetricsRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).get_metrics(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = get_metricsSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/rpc_models.ClusterRpc/mutate_state" => {
                    #[allow(non_camel_case_types)]
                    struct mutate_stateSvc<T: ClusterRpc>(pub Arc<T>);
                    impl<T: ClusterRpc> tonic::server::UnaryService<super::MutateRequest>
                    for mutate_stateSvc<T> {
                        type Response = super::RpcResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::MutateRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).mutate_state(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = mutate_stateSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => {
                    Box::pin(async move {
                        Ok(
                            http::Response::builder()
                                .status(200)
                                .header("grpc-status", "12")
                                .header("content-type", "application/grpc")
                                .body(empty_body())
                                .unwrap(),
                        )
                    })
                }
            }
        }
    }
    impl<T: ClusterRpc> Clone for ClusterRpcServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
            }
        }
    }
    impl<T: ClusterRpc> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(self.0.clone())
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: ClusterRpc> tonic::server::NamedService for ClusterRpcServer<T> {
        const NAME: &'static str = "rpc_models.ClusterRpc";
    }
}

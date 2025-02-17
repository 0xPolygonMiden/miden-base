// This file is @generated by prost-build.
/// Request message for proof generation containing payload and proof type metadata.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ProvingRequest {
    /// Type of proof being requested, determines payload interpretation
    #[prost(enumeration = "ProofType", tag = "1")]
    pub proof_type: i32,
    /// Serialized payload requiring proof generation. The encoding format is
    /// type-specific:
    /// - TRANSACTION: TransactionWitness encoded.
    /// - BATCH: ProposedBatch encoded.
    /// - BLOCK: ProposedBlock encoded.
    #[prost(bytes = "vec", tag = "2")]
    pub payload: ::prost::alloc::vec::Vec<u8>,
}
/// Response message containing the generated proof.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ProvingResponse {
    /// Serialized proof bytes.
    /// - TRANSACTION: Returns an encoded ProvenTransaction.
    /// - BATCH: Returns an encoded ProvenBatch.
    /// - BLOCK: Returns an encoded ProvenBlock.
    #[prost(bytes = "vec", tag = "1")]
    pub payload: ::prost::alloc::vec::Vec<u8>,
}
/// Enumeration of supported proof types.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum ProofType {
    /// Proof for a single transaction.
    Transaction = 0,
    /// Proof covering a batch of transactions.
    Batch = 1,
    /// Proof for entire block validity.
    Block = 2,
}
impl ProofType {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            Self::Transaction => "TRANSACTION",
            Self::Batch => "BATCH",
            Self::Block => "BLOCK",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "TRANSACTION" => Some(Self::Transaction),
            "BATCH" => Some(Self::Batch),
            "BLOCK" => Some(Self::Block),
            _ => None,
        }
    }
}
/// Generated client implementations.
pub mod api_client {
    #![allow(
        unused_variables,
        dead_code,
        missing_docs,
        clippy::wildcard_imports,
        clippy::let_unit_value,
    )]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    #[derive(Debug, Clone)]
    pub struct ApiClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl<T> ApiClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + core::marker::Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + core::marker::Send,
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
        ) -> ApiClient<InterceptedService<T, F>>
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
            >>::Error: Into<StdError> + core::marker::Send + core::marker::Sync,
        {
            ApiClient::new(InterceptedService::new(inner, interceptor))
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
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_decoding_message_size(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_encoding_message_size(limit);
            self
        }
        /// Generates a proof for the requested payload.
        pub async fn prove(
            &mut self,
            request: impl tonic::IntoRequest<super::ProvingRequest>,
        ) -> core::result::Result<
            tonic::Response<super::ProvingResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::unknown(
                        alloc::format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/proving_service.Api/Prove",
            );
            let mut req = request.into_request();
            req.extensions_mut().insert(GrpcMethod::new("proving_service.Api", "Prove"));
            self.inner.unary(req, path, codec).await
        }
    }
}

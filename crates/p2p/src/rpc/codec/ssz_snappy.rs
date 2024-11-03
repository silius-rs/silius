use crate::rpc::{
    handler::Error,
    methods::{
        GoodbyeReason, MetaData, MetaDataRequest, Ping, PooledUserOpHashesRequest,
        PooledUserOpHashesResponse, PooledUserOpsByHashRequest, PooledUserOpsByHashResponse,
        RPCResponse, Status,
    },
    outbound::OutboundRequest,
    protocol::{InboundRequest, Protocol, ProtocolId},
};
use ssz_rs::{Deserialize, Serialize};
use std::io::{self, Read, Write};
use tokio_util::{
    bytes::BytesMut,
    codec::{Decoder, Encoder},
};
use tracing::trace;
use unsigned_varint::codec::Uvi;

pub struct SSZSnappyInboundCodec {
    protocol: ProtocolId,
    inner: Uvi<usize>,
}

impl SSZSnappyInboundCodec {
    pub fn new(protocol: ProtocolId) -> Self {
        let uvi_codec = Uvi::default();

        Self { protocol, inner: uvi_codec }
    }
}

impl Encoder<RPCResponse> for SSZSnappyInboundCodec {
    type Error = Error;

    fn encode(&mut self, item: RPCResponse, dst: &mut BytesMut) -> Result<(), Self::Error> {
        trace!("Inbound response {:?}", item);

        // response_chunk ::= <result> | <encoding-dependent-header> | <encoded-payload>

        // encode <result>
        // TODO: for now let's add 0, but we should handle other cases
        dst.extend_from_slice(&[0]);

        let ssz_bytes = item.serialize()?;

        // encode <encoding-dependent-header>
        self.inner.encode(ssz_bytes.len(), dst)?;

        // encode <encoded-payload>
        let mut writer = snap::write::FrameEncoder::new(vec![]);
        writer.write_all(&ssz_bytes)?;
        let compressed_data = writer.into_inner().map_err(|e| Error::IoError(e.into_error()))?;
        dst.extend_from_slice(&compressed_data);

        trace!("Inbound response buffer {:?}", dst);

        Ok(())
    }
}

impl Decoder for SSZSnappyInboundCodec {
    type Item = InboundRequest;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // MetaData request would send empty content.
        // https://github.com/eth-infinitism/bundler-spec/blob/main/p2p-specs/p2p-interface.md#getmetadata
        if src.is_empty() && !matches!(self.protocol.protocol, Protocol::MetaData) {
            return Err(Error::IoError(io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "No data received",
            )));
        }

        // decode header
        let _ = self.inner.decode(src)?; // TODO: use length to verify size

        // decode payload
        let mut buffer = vec![];
        snap::read::FrameDecoder::<&[u8]>::new(src).read_to_end(&mut buffer)?;

        let request = match self.protocol.protocol {
            Protocol::Status => InboundRequest::Status(Status::deserialize(&buffer)?),
            Protocol::Goodbye => InboundRequest::Goodbye(GoodbyeReason::deserialize(&buffer)?),
            Protocol::Ping => InboundRequest::Ping(Ping::deserialize(&buffer)?),
            Protocol::MetaData => InboundRequest::MetaData(MetaDataRequest::deserialize(&buffer)?),
            Protocol::PooledUserOpHashes => {
                InboundRequest::PooledUserOpHashes(PooledUserOpHashesRequest::deserialize(&buffer)?)
            }
            Protocol::PooledUserOpsByHash => InboundRequest::PooledUserOpsByHash(
                PooledUserOpsByHashRequest::deserialize(&buffer)?,
            ),
        };

        trace!("Inbound request {:?}", request);

        Ok(request.into())
    }
}

pub struct SSZSnappyOutboundCodec {
    protocol: ProtocolId,
    inner: Uvi<usize>,
}

impl SSZSnappyOutboundCodec {
    pub fn new(protocol: ProtocolId) -> Self {
        let uvi_codec = Uvi::default();

        Self { protocol, inner: uvi_codec }
    }
}

impl Encoder<OutboundRequest> for SSZSnappyOutboundCodec {
    type Error = Error;

    fn encode(&mut self, item: OutboundRequest, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let mut buffer = vec![];

        trace!("Outbound request {:?}", item);

        match item {
            OutboundRequest::Status(status) => status.serialize(&mut buffer)?,
            OutboundRequest::Goodbye(reason) => reason.serialize(&mut buffer)?,
            OutboundRequest::Ping(ping) => ping.serialize(&mut buffer)?,
            OutboundRequest::MetaData(metadata) => metadata.serialize(&mut buffer)?,
            OutboundRequest::PooledUserOpHashes(pooled_user_op_hashes_req) => {
                pooled_user_op_hashes_req.serialize(&mut buffer)?
            }

            OutboundRequest::PooledUserOpsByHash(pooled_user_ops_by_hash_req) => {
                pooled_user_ops_by_hash_req.serialize(&mut buffer)?
            }
        };

        // encode header
        self.inner.encode(buffer.len(), dst)?;

        // encode payload
        let mut writer = snap::write::FrameEncoder::new(vec![]);
        writer.write_all(&buffer)?;
        let compressed_data = writer.into_inner().map_err(|e| Error::IoError(e.into_error()))?;
        dst.extend_from_slice(&compressed_data);

        trace!("Outbound request buffer {:?}", dst);

        Ok(())
    }
}

impl Decoder for SSZSnappyOutboundCodec {
    type Item = RPCResponse;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // response_chunk ::= <result> | <encoding-dependent-header> | <encoded-payload>

        // TODO: response chunks

        // decode <result>
        let _ = src.split_to(1); // TODO: handle result

        // decode <encoding-dependent-header>
        let _ = self.inner.decode(src)?; // TODO: use length to verify size

        // decode <encoded-payload>
        let mut decompressed_data = vec![];
        snap::read::FrameDecoder::<&[u8]>::new(src).read_to_end(&mut decompressed_data)?;

        let response = match self.protocol.protocol {
            Protocol::Status => RPCResponse::Status(Status::deserialize(&decompressed_data)?),
            Protocol::Goodbye => {
                RPCResponse::Goodbye(GoodbyeReason::deserialize(&decompressed_data)?)
            }
            Protocol::Ping => RPCResponse::Pong(Ping::deserialize(&decompressed_data)?),
            Protocol::MetaData => RPCResponse::MetaData(MetaData::deserialize(&decompressed_data)?),
            Protocol::PooledUserOpHashes => RPCResponse::PooledUserOpHashes(
                PooledUserOpHashesResponse::deserialize(&decompressed_data)?,
            ),
            Protocol::PooledUserOpsByHash => RPCResponse::PooledUserOpsByHash(
                PooledUserOpsByHashResponse::deserialize(&decompressed_data)?,
            ),
        };

        trace!("Outbound response {:?}", response);

        Ok(response.into())
    }
}

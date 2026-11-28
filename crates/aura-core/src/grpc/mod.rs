//! Pure-Rust `.proto` parser, dynamic Protobuf wire encoder/decoder,
//! gRPC length-prefixed framing, and FileDescriptorProto / ServerReflection codecs.

pub mod codec;
pub mod parser;
pub mod reflection;
pub mod schema;

pub use codec::{
    decode_protobuf_to_json, encode_json_to_protobuf, frame_grpc_message, unframe_grpc_messages,
};
pub use parser::{parse_proto_file, sample_json_for_message};
pub use reflection::{
    ServerReflectionRequestKind, ServerReflectionResult, decode_file_descriptor_proto_bytes,
    decode_server_reflection_response, encode_server_reflection_request,
};
pub use schema::{
    ProtoEnum, ProtoField, ProtoMessage, ProtoMethod, ProtoSchema, ProtoService, RpcCardinality,
};

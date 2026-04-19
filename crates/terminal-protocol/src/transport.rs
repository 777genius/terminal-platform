use std::{env, io, path::PathBuf};

use bytes::Bytes;
use interprocess::local_socket::{
    GenericFilePath, GenericNamespaced, Name, NameType as _, ToFsName as _, ToNsName as _,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{ProtocolError, ResponseEnvelope};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalSocketAddress {
    Namespaced(String),
    Filesystem(PathBuf),
}

impl LocalSocketAddress {
    #[must_use]
    pub fn from_runtime_slug(slug: impl Into<String>) -> Self {
        let slug = slug.into();

        if GenericNamespaced::is_supported() {
            Self::Namespaced(slug)
        } else {
            Self::Filesystem(env::temp_dir().join(slug))
        }
    }

    pub fn to_name(&self) -> io::Result<Name<'_>> {
        match self {
            Self::Namespaced(value) => value.as_str().to_ns_name::<GenericNamespaced>(),
            Self::Filesystem(path) => path.as_os_str().to_fs_name::<GenericFilePath>(),
        }
    }
}

impl std::fmt::Display for LocalSocketAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Namespaced(value) => write!(f, "{value}"),
            Self::Filesystem(path) => write!(f, "{}", path.display()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", content = "value")]
pub enum TransportResponse {
    Response(Box<ResponseEnvelope>),
    Error(ProtocolError),
}

impl TransportResponse {
    #[must_use]
    pub fn from_result(result: Result<ResponseEnvelope, ProtocolError>) -> Self {
        match result {
            Ok(response) => Self::Response(Box::new(response)),
            Err(error) => Self::Error(error),
        }
    }

    pub fn into_result(self) -> Result<ResponseEnvelope, ProtocolError> {
        match self {
            Self::Response(response) => Ok(*response),
            Self::Error(error) => Err(error),
        }
    }
}

pub fn encode_json_frame<T>(value: &T) -> Result<Bytes, ProtocolError>
where
    T: Serialize,
{
    serde_json::to_vec(value).map(Bytes::from).map_err(ProtocolError::serialize)
}

pub fn decode_json_frame<T>(frame: &[u8]) -> Result<T, ProtocolError>
where
    T: DeserializeOwned,
{
    serde_json::from_slice(frame).map_err(ProtocolError::deserialize)
}

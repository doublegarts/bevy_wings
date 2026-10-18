//! Encoding values for storage: [`Codec`] and [`JsonCodec`].

use serde::Serialize;
use serde::de::DeserializeOwned;

use bevy_wings_domain::DomainError;

/// Turns values into bytes and back.
pub trait Codec<T>: Send + Sync {
    /// Encodes a value.
    fn encode(&self, value: &T) -> Result<Vec<u8>, DomainError>;
    /// Decodes a value.
    fn decode(&self, bytes: &[u8]) -> Result<T, DomainError>;
}

/// JSON through serde. Readable on disk and stable across versions of your app as long as
/// the field names stay.
#[derive(Debug, Clone, Copy, Default)]
pub struct JsonCodec {
    /// Indent the output.
    pub pretty: bool,
}

impl JsonCodec {
    /// Indented JSON.
    pub fn pretty() -> Self {
        Self { pretty: true }
    }
}

impl<T: Serialize + DeserializeOwned> Codec<T> for JsonCodec {
    fn encode(&self, value: &T) -> Result<Vec<u8>, DomainError> {
        let r = if self.pretty {
            serde_json::to_vec_pretty(value)
        } else {
            serde_json::to_vec(value)
        };
        r.map_err(|e| DomainError::Unexpected(format!("encode: {e}")))
    }

    fn decode(&self, bytes: &[u8]) -> Result<T, DomainError> {
        serde_json::from_slice(bytes).map_err(|e| DomainError::Unexpected(format!("decode: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Values round trip.
    #[test]
    fn json_round_trip() {
        let c = JsonCodec::pretty();
        let bytes = Codec::<Vec<u32>>::encode(&c, &vec![1, 2]).unwrap();
        assert_eq!(Codec::<Vec<u32>>::decode(&c, &bytes).unwrap(), vec![1, 2]);
        assert!(Codec::<Vec<u32>>::decode(&c, b"nope").is_err());
    }
}

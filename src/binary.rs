use pumpkin_plugin_api::Context;
use serde::{Deserialize, Serialize};
use crate::storage::{self, DataCodec};

pub trait BinaryConfig: Serialize + for<'de> Deserialize<'de> + Default {
}

/// [`DataCodec`] that (de)serializes values as compact postcard binary.
pub struct PostcardCodec;

impl<T> DataCodec<T> for PostcardCodec
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    fn encode(value: &T) -> Result<Vec<u8>, String> {
        postcard::to_allocvec(value).map_err(|err| format!("Failed to serialize binary file: {err}"))
    }

    fn decode(data: &[u8]) -> Result<T, String> {
        postcard::from_bytes(data).map_err(|err| format!("Failed to parse binary file: {err}"))
    }
}

pub fn load_binary<T: BinaryConfig>(context: &Context, file_name: &str) -> Result<T, String> {
    storage::load_data::<T, PostcardCodec>(context, file_name)
}

pub fn load_binary_with_dir_and_filename<T: BinaryConfig>(context: &Context, dir: &str, file_name: &str) -> Result<T, String> {
    storage::load_data_with_dir_and_filename::<T, PostcardCodec>(context, dir, file_name)
}

pub fn save_binary<T: BinaryConfig>(context: &Context, file_name: &str, value: &T) -> Result<(), String> {
    storage::save_data::<T, PostcardCodec>(context, file_name, value)
}

pub fn save_binary_with_dir_and_filename<T: BinaryConfig>(context: &Context, dir: &str, file_name: &str, value: &T) -> Result<(), String> {
    storage::save_data_with_dir_and_filename::<T, PostcardCodec>(context, dir, file_name, value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize, Deserialize, Default, Debug, PartialEq)]
    struct DummyBinary {
        name: String,
        value: i32,
    }

    impl BinaryConfig for DummyBinary {}

    #[test]
    fn test_postcard_codec_round_trip() {
        let value = DummyBinary {
            name: "hoge".into(),
            value: 42,
        };

        let encoded = PostcardCodec::encode(&value).unwrap();
        let decoded: DummyBinary = PostcardCodec::decode(&encoded).unwrap();

        assert_eq!(decoded, value);
    }

    #[test]
    fn test_postcard_codec_decode_returns_err_on_invalid_data() {
        let result: Result<DummyBinary, String> = PostcardCodec::decode(&[0xff, 0xff, 0xff]);

        assert!(result.is_err());
        assert!(result.unwrap_err().starts_with("Failed to parse binary file"));
    }
}

use pumpkin_plugin_api::Context;
use serde::{Deserialize, Serialize};
use crate::storage::{self, DataCodec};

pub trait PluginConfig: Serialize + for<'de> Deserialize<'de> + Default {
}

/// [`DataCodec`] that (de)serializes values as pretty-printed TOML.
pub struct TomlCodec;

impl<T> DataCodec<T> for TomlCodec
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    fn encode(value: &T) -> Result<Vec<u8>, String> {
        toml::to_string_pretty(value)
            .map(|s| s.into_bytes())
            .map_err(|err| format!("Failed to serialize config file: {err}"))
    }

    fn decode(data: &[u8]) -> Result<T, String> {
        let text = std::str::from_utf8(data)
            .map_err(|err| format!("Config file is not valid UTF-8: {err}"))?;
        toml::from_str(text).map_err(|err| format!("Failed to parse config file: {err}"))
    }
}

pub fn load_config<T: PluginConfig>(context: &Context, file_name: &str) -> Result<T, String> {
    storage::load_data::<T, TomlCodec>(context, file_name)
}

pub fn load_config_with_dir_and_filename<T: PluginConfig>(context: &Context, config_dir: &str, file_name: &str) -> Result<T, String> {
    storage::load_data_with_dir_and_filename::<T, TomlCodec>(context, config_dir, file_name)
}

pub fn save_config<T: PluginConfig>(context: &Context, file_name: &str, value: &T) -> Result<(), String> {
    storage::save_data::<T, TomlCodec>(context, file_name, value)
}

pub fn save_config_with_dir_and_filename<T: PluginConfig>(context: &Context, config_dir: &str, file_name: &str, value: &T) -> Result<(), String> {
    storage::save_data_with_dir_and_filename::<T, TomlCodec>(context, config_dir, file_name, value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize, Deserialize, Default, Debug, PartialEq)]
    struct DummyConfig {
        name: String,
        value: i32,
    }

    impl PluginConfig for DummyConfig {}

    #[test]
    fn test_toml_codec_round_trip() {
        let config = DummyConfig {
            name: "hoge".into(),
            value: 42,
        };

        let encoded = TomlCodec::encode(&config).unwrap();
        let decoded: DummyConfig = TomlCodec::decode(&encoded).unwrap();

        assert_eq!(decoded, config);
    }

    #[test]
    fn test_toml_codec_decode_reads_plain_toml() {
        let decoded: DummyConfig = TomlCodec::decode(b"name = \"hoge\"\nvalue = 42\n").unwrap();

        assert_eq!(
            decoded,
            DummyConfig {
                name: "hoge".into(),
                value: 42,
            }
        );
    }

    #[test]
    fn test_toml_codec_decode_returns_err_on_invalid_toml() {
        let result: Result<DummyConfig, String> = TomlCodec::decode(b"not valid toml =");

        assert!(result.is_err());
        assert!(result.unwrap_err().starts_with("Failed to parse config file"));
    }

    #[test]
    fn test_toml_codec_decode_returns_err_on_invalid_utf8() {
        let result: Result<DummyConfig, String> = TomlCodec::decode(&[0xff, 0xfe, 0xfd]);

        assert!(result.is_err());
        assert!(result.unwrap_err().starts_with("Config file is not valid UTF-8"));
    }
}

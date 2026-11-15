//! Generic "load from / save to the plugin's data folder" helpers.
//!
//! Path resolution, directory creation and the read-or-create-default
//! behavior are shared across any file format. The actual byte<->value
//! conversion is supplied by a [`DataCodec`] implementation (see
//! [`crate::config::TomlCodec`] or [`crate::binary::RawBytesCodec`]).

use std::path::PathBuf;
use pumpkin_plugin_api::Context;

/// Converts a value to and from the bytes stored on disk.
pub trait DataCodec<T> {
    fn encode(value: &T) -> Result<Vec<u8>, String>;
    fn decode(data: &[u8]) -> Result<T, String>;
}

pub fn load_data<T: Default, C: DataCodec<T>>(context: &Context, file_name: &str) -> Result<T, String> {
    load_data_with_dir_and_filename::<T, C>(context, "", file_name)
}

pub fn load_data_with_dir_and_filename<T: Default, C: DataCodec<T>>(
    context: &Context,
    dir: &str,
    file_name: &str,
) -> Result<T, String> {
    let file_path = construct_file_path(context.get_data_folder(), dir, file_name)?;
    create_parent_directory_if_not_exists(&file_path);
    read_or_create_file::<T, C>(&file_path)
}

pub fn save_data<T, C: DataCodec<T>>(context: &Context, file_name: &str, value: &T) -> Result<(), String> {
    save_data_with_dir_and_filename::<T, C>(context, "", file_name, value)
}

pub fn save_data_with_dir_and_filename<T, C: DataCodec<T>>(
    context: &Context,
    dir: &str,
    file_name: &str,
    value: &T,
) -> Result<(), String> {
    let file_path = construct_file_path(context.get_data_folder(), dir, file_name)?;
    create_parent_directory_if_not_exists(&file_path);
    let data = C::encode(value)?;
    std::fs::write(&file_path, data).map_err(|err| format!("Failed to write file: {err}"))
}

fn read_or_create_file<T: Default, C: DataCodec<T>>(file_path: &PathBuf) -> Result<T, String> {
    if !file_path.exists() {
        let default_value = T::default();
        let data = C::encode(&default_value)?;

        if let Err(err) = std::fs::write(file_path, data) {
            return Err(format!("Failed to write default file: {err}"));
        }
    }

    let data = match std::fs::read(file_path) {
        Ok(data) => data,
        Err(err) => return Err(format!("Failed to read file: {err}")),
    };
    C::decode(&data)
}

fn construct_file_path(data_folder_path: String, dir: &str, file_name: &str) -> Result<PathBuf, String> {
    let dir = remove_slash_prefix(dir);
    let file_name = remove_slash_prefix(file_name);

    if dir.starts_with("./") || dir.starts_with("../") {
        return Err("Relative paths are not allowed in dir".into());
    }

    let full_path = data_folder_path + "/" + dir + "/" + file_name;
    Ok(PathBuf::from(full_path))
}

fn create_parent_directory_if_not_exists(file_path: &PathBuf) {
    if let Some(parent_dir) = file_path.parent() {
        if !parent_dir.exists() {
            if let Err(err) = std::fs::create_dir_all(parent_dir) {
                tracing::warn!("Failed to create data directory: {err}");
            }
        }
    }
}

fn remove_slash_prefix(s: &str) -> &str {
    s.strip_prefix("/").unwrap_or(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[test]
    fn test_remove_slash_prefix() {
        assert_eq!(remove_slash_prefix("/test/hoge/"), "test/hoge/");
        assert_eq!(remove_slash_prefix("test/hoge/"), "test/hoge/");
    }

    #[derive(Debug, Default, PartialEq)]
    struct DummyValue {
        name: String,
        value: i32,
    }

    struct DummyCodec;

    impl DataCodec<DummyValue> for DummyCodec {
        fn encode(value: &DummyValue) -> Result<Vec<u8>, String> {
            Ok(format!("{}:{}", value.name, value.value).into_bytes())
        }

        fn decode(data: &[u8]) -> Result<DummyValue, String> {
            let text = std::str::from_utf8(data).map_err(|err| err.to_string())?;
            let (name, value) = text
                .split_once(':')
                .ok_or_else(|| "missing ':'".to_string())?;
            let value = value.parse::<i32>().map_err(|err| err.to_string())?;
            Ok(DummyValue {
                name: name.to_string(),
                value,
            })
        }
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!(
            "pumpkin_plugin_toolkit_storage_test_{}_{}_{}",
            std::process::id(),
            label,
            id
        ))
    }

    #[test]
    fn test_read_or_create_file_creates_default_when_missing() {
        let dir = unique_temp_dir("creates_default");
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("data.bin");

        let result: Result<DummyValue, String> = read_or_create_file::<DummyValue, DummyCodec>(&file_path);

        assert_eq!(result, Ok(DummyValue::default()));
        assert!(file_path.exists());

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_read_or_create_file_reads_existing_file() {
        let dir = unique_temp_dir("reads_existing");
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("data.bin");
        std::fs::write(&file_path, "hoge:42").unwrap();

        let result: Result<DummyValue, String> = read_or_create_file::<DummyValue, DummyCodec>(&file_path);

        assert_eq!(
            result,
            Ok(DummyValue {
                name: "hoge".into(),
                value: 42,
            })
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_read_or_create_file_returns_err_on_invalid_data() {
        let dir = unique_temp_dir("invalid_data");
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("data.bin");
        std::fs::write(&file_path, "not valid").unwrap();

        let result: Result<DummyValue, String> = read_or_create_file::<DummyValue, DummyCodec>(&file_path);

        assert!(result.is_err());

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_read_or_create_file_returns_err_when_write_fails() {
        let dir = unique_temp_dir("write_fails");
        let file_path = dir.join("missing_subdir").join("data.bin");

        let result: Result<DummyValue, String> = read_or_create_file::<DummyValue, DummyCodec>(&file_path);

        assert!(result.is_err());
        assert!(result.unwrap_err().starts_with("Failed to write default file"));
    }

    #[test]
    fn test_construct_file_path_joins_segments() {
        let result = construct_file_path("/data".to_string(), "plugins/hoge", "data.bin");

        assert_eq!(result, Ok(PathBuf::from("/data/plugins/hoge/data.bin")));
    }

    #[test]
    fn test_construct_file_path_strips_leading_slashes() {
        let result = construct_file_path("/data".to_string(), "/plugins/hoge", "/data.bin");

        assert_eq!(result, Ok(PathBuf::from("/data/plugins/hoge/data.bin")));
    }

    #[test]
    fn test_construct_file_path_rejects_dot_slash_dir() {
        let result = construct_file_path("/data".to_string(), "./plugins/hoge", "data.bin");

        assert_eq!(result, Err("Relative paths are not allowed in dir".to_string()));
    }

    #[test]
    fn test_construct_file_path_rejects_parent_dir() {
        let result = construct_file_path("/data".to_string(), "../plugins/hoge", "data.bin");

        assert_eq!(result, Err("Relative paths are not allowed in dir".to_string()));
    }

    #[test]
    fn test_create_parent_directory_if_not_exists_creates_missing_parent() {
        let dir = unique_temp_dir("create_dir_missing");
        let file_path = dir.join("nested").join("data.bin");

        create_parent_directory_if_not_exists(&file_path);

        assert!(file_path.parent().unwrap().is_dir());

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_create_parent_directory_if_not_exists_keeps_existing_parent() {
        let dir = unique_temp_dir("create_dir_existing");
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("data.bin");
        std::fs::write(&file_path, "hoge:1").unwrap();

        create_parent_directory_if_not_exists(&file_path);

        assert!(file_path.is_file());

        std::fs::remove_dir_all(&dir).unwrap();
    }
}

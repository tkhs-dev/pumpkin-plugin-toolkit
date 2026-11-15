use std::path::{Path, PathBuf};
use pumpkin_plugin_api::Context;
use serde::{Deserialize, Serialize};

pub trait PluginConfig: Serialize + for<'de> Deserialize<'de> + Default {
}

pub fn load_config<T: PluginConfig>(context: &Context, file_name: &str) -> Result<T, String> {
    load_config_with_dir_and_filename(context, "", file_name)
}

pub fn load_config_with_dir_and_filename<T: PluginConfig>(context: &Context, config_dir: &str, file_name: &str) -> Result<T, String> {
    let config_path = construct_config_path(context.get_data_folder(), config_dir, file_name)?;
    create_config_directory_if_not_exists(&config_path);
    read_or_create_config_file(&config_path)
}

fn read_or_create_config_file<T: PluginConfig>(config_path: &PathBuf) -> Result<T, String> {
    if !config_path.exists() {
        let default_config = T::default();
        let config_data = toml::to_string_pretty(&default_config).unwrap_or_default();

        if let Err(err) = std::fs::write(config_path, config_data) {
            return Err(format!("Failed to write default config file: {err}"));
        }
    }

    let config_data = match std::fs::read_to_string(config_path) {
        Ok(data) => data,
        Err(err) => return Err(format!("Failed to read config file: {err}")),
    };
    toml::from_str::<T>(&config_data).map_err(|err| format!("Failed to parse config file: {err}"))
}

fn construct_config_path(data_folder_path: String, config_dir: &str, file_name: &str) -> Result<PathBuf, String> {
    let config_dir = remove_slash_prefix(config_dir);
    let file_name = remove_slash_prefix(file_name);

    if config_dir.starts_with("./") || config_dir.starts_with("../") {
        return Err(
            "Relative paths are not allowed in config_dir".into()
        );
    }

    let full_path = data_folder_path + "/" + &config_dir + "/" + file_name;
    Ok(PathBuf::from(full_path))
}

fn create_config_directory_if_not_exists(config_path: &PathBuf) {
    if let Some(parent_dir) = config_path.parent() {
        if !parent_dir.exists() {
            if let Err(err) = std::fs::create_dir_all(parent_dir) {
                tracing::warn!("Failed to create config directory: {err}");
            }
        }
    }
}

fn remove_slash_prefix(s: &str) -> &str {
    s.strip_prefix("/").unwrap_or(s)
}

fn remove_slash_suffix(s: &str) -> &str {
    s.strip_suffix("/").unwrap_or(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[test]
    fn test_remove_slash_prefix() {
        assert_eq!(remove_slash_prefix("/test/hoge/"), "test/hoge/");
        assert_eq!(remove_slash_prefix("test/hoge/"), "test/hoge/");;
    }

    #[test]
    fn test_remove_slash_suffix() {
        assert_eq!(remove_slash_suffix("/test/hoge/"), "/test/hoge");
        assert_eq!(remove_slash_suffix("/test/hoge"), "/test/hoge");
    }

    #[derive(Serialize, Deserialize, Default, Debug, PartialEq)]
    struct DummyConfig {
        name: String,
        value: i32,
    }

    impl PluginConfig for DummyConfig {}

    fn unique_temp_dir(label: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!(
            "pumpkin_plugin_toolkit_test_{}_{}_{}",
            std::process::id(),
            label,
            id
        ))
    }

    #[test]
    fn test_read_or_create_config_file_creates_default_when_missing() {
        let dir = unique_temp_dir("creates_default");
        std::fs::create_dir_all(&dir).unwrap();
        let config_path = dir.join("config.toml");

        let result: Result<DummyConfig, String> = read_or_create_config_file(&config_path);

        assert_eq!(result, Ok(DummyConfig::default()));
        assert!(config_path.exists());

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_read_or_create_config_file_reads_existing_file() {
        let dir = unique_temp_dir("reads_existing");
        std::fs::create_dir_all(&dir).unwrap();
        let config_path = dir.join("config.toml");
        std::fs::write(&config_path, "name = \"hoge\"\nvalue = 42\n").unwrap();

        let result: Result<DummyConfig, String> = read_or_create_config_file(&config_path);

        assert_eq!(
            result,
            Ok(DummyConfig {
                name: "hoge".into(),
                value: 42,
            })
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_read_or_create_config_file_returns_err_on_invalid_toml() {
        let dir = unique_temp_dir("invalid_toml");
        std::fs::create_dir_all(&dir).unwrap();
        let config_path = dir.join("config.toml");
        std::fs::write(&config_path, "not valid toml =").unwrap();

        let result: Result<DummyConfig, String> = read_or_create_config_file(&config_path);

        assert!(result.is_err());
        assert!(result.unwrap_err().starts_with("Failed to parse config file"));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_read_or_create_config_file_returns_err_when_write_fails() {
        let dir = unique_temp_dir("write_fails");
        let config_path = dir.join("missing_subdir").join("config.toml");

        let result: Result<DummyConfig, String> = read_or_create_config_file(&config_path);

        assert!(result.is_err());
        assert!(result.unwrap_err().starts_with("Failed to write default config file"));
    }

    #[test]
    fn test_construct_config_path_joins_segments() {
        let result = construct_config_path("/data".to_string(), "plugins/hoge", "config.toml");

        assert_eq!(
            result,
            Ok(PathBuf::from("/data/plugins/hoge/config.toml"))
        );
    }

    #[test]
    fn test_construct_config_path_strips_leading_slashes() {
        let result = construct_config_path("/data".to_string(), "/plugins/hoge", "/config.toml");

        assert_eq!(
            result,
            Ok(PathBuf::from("/data/plugins/hoge/config.toml"))
        );
    }

    #[test]
    fn test_construct_config_path_rejects_dot_slash_config_dir() {
        let result = construct_config_path("/data".to_string(), "./plugins/hoge", "config.toml");

        assert_eq!(
            result,
            Err("Relative paths are not allowed in config_dir".to_string())
        );
    }

    #[test]
    fn test_construct_config_path_rejects_parent_dir_config_dir() {
        let result = construct_config_path("/data".to_string(), "../plugins/hoge", "config.toml");

        assert_eq!(
            result,
            Err("Relative paths are not allowed in config_dir".to_string())
        );
    }

    #[test]
    fn test_create_config_directory_if_not_exists_creates_missing_parent() {
        let dir = unique_temp_dir("create_dir_missing");
        let config_path = dir.join("nested").join("config.toml");

        create_config_directory_if_not_exists(&config_path);

        assert!(config_path.parent().unwrap().is_dir());

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_create_config_directory_if_not_exists_keeps_existing_parent() {
        let dir = unique_temp_dir("create_dir_existing");
        std::fs::create_dir_all(&dir).unwrap();
        let config_path = dir.join("config.toml");
        std::fs::write(&config_path, "name = \"hoge\"\nvalue = 1\n").unwrap();

        create_config_directory_if_not_exists(&config_path);

        assert!(config_path.is_file());

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
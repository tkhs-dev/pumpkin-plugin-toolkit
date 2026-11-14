use std::path::{Path, PathBuf};
use pumpkin_plugin_api::Context;
use serde::{Deserialize, Serialize};

trait PluginConfig: Serialize + for<'de> Deserialize<'de> + Default {
}

pub fn load_config<T: PluginConfig>(context: &Context, config_dir: &str, file_name: &str) -> Result<T, String> {
    let config_path = construct_config_path(context, config_dir, file_name)?;
    create_config_directory_if_not_exists(context, &config_path);
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

fn construct_config_path(context: &Context, config_dir: &str, file_name: &str) -> Result<PathBuf, String> {
    let config_dir = remove_slash_prefix(config_dir);
    let file_name = remove_slash_prefix(file_name);

    if config_dir.starts_with("./") || config_dir.starts_with("../") {
        return Err(
            "Relative paths are not allowed in config_dir".into()
        );
    }

    let full_path = context.get_data_folder() + "/" + &config_dir + "/" + file_name;
    Ok(PathBuf::from(full_path))
}

fn create_config_directory_if_not_exists(context: &Context, config_path: &PathBuf) {
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
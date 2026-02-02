// SPDX-License-Identifier: MPL-2.0

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub uri: String,
    pub favourite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelList {
    pub channels: Vec<Channel>,
}

#[derive(Error, Debug)]
pub enum ChannelError {
    #[error("Failed to read config file: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("Failed to parse TOML: {0}")]
    ParseError(#[from] toml::de::Error),
    #[error("Failed to serialize TOML: {0}")]
    SerializeError(#[from] toml::ser::Error),
}

/// Returns the config directory path
fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("cosmic-internet-radio-applet")
}

/// Returns the full path to the channels.toml file
fn channels_file_path() -> PathBuf {
    config_dir().join("channels.toml")
}

/// Ensures the config directory exists
fn ensure_config_dir() -> Result<(), ChannelError> {
    let dir = config_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }
    Ok(())
}

/// Returns the default channel list
pub fn default_channels() -> ChannelList {
    let toml_str = include_str!("../resources/default_channels.toml");
    toml::from_str(toml_str).unwrap_or_default()
}

/// Load channels from the config file.
/// If the file doesn't exist, creates it with default channels.
pub fn load_channels() -> Result<ChannelList, ChannelError> {
    let path = channels_file_path();

    if !path.exists() {
        // First run - create default channels
        let defaults = default_channels();
        save_channels(&defaults)?;
        return Ok(defaults);
    }

    let content = fs::read_to_string(&path)?;
    let list: ChannelList = toml::from_str(&content)?;
    Ok(list)
}

/// Save channels to the config file
pub fn save_channels(list: &ChannelList) -> Result<(), ChannelError> {
    ensure_config_dir()?;
    let path = channels_file_path();
    let content = toml::to_string_pretty(list)?;
    fs::write(&path, content)?;
    Ok(())
}

/// Get the path to the channels file (for error messages)
pub fn get_channels_file_path() -> PathBuf {
    channels_file_path()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_list_serialization() {
        let list = ChannelList {
            channels: vec![
                Channel {
                    id: "fip-radio".to_string(),
                    name: "FIP Radio".to_string(),
                    uri: "http://icecast.radiofrance.fr/fip-midfi.mp3".to_string(),
                    favourite: true,
                },
                Channel {
                    id: "groove-salad".to_string(),
                    name: "Groove Salad".to_string(),
                    uri: "https://somafm.com/groovesalad256.pls".to_string(),
                    favourite: false,
                },
            ],
        };

        let toml_str = toml::to_string_pretty(&list).unwrap();
        let parsed: ChannelList = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.channels.len(), 2);
        assert_eq!(parsed.channels[0].name, "FIP Radio");
        assert_eq!(parsed.channels[1].favourite, false);
    }
}

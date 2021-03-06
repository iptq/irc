//! JSON configuration files using serde
use std::borrow::ToOwned;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::{Error, ErrorKind};
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::Path;

#[cfg(feature = "json")]
use serde_json;
#[cfg(feature = "yaml")]
use serde_yaml;
#[cfg(feature = "toml")]
use toml;

use error;
use error::{Result, ResultExt};

/// Configuration data.
#[derive(Clone, Deserialize, Serialize, Default, PartialEq, Debug)]
pub struct Config {
    /// A list of the owners of the client by nickname (for bots).
    pub owners: Option<Vec<String>>,
    /// The client's nickname.
    pub nickname: Option<String>,
    /// The client's NICKSERV password.
    pub nick_password: Option<String>,
    /// Alternative nicknames for the client, if the default is taken.
    pub alt_nicks: Option<Vec<String>>,
    /// The client's username.
    pub username: Option<String>,
    /// The client's real name.
    pub realname: Option<String>,
    /// The server to connect to.
    pub server: Option<String>,
    /// The port to connect on.
    pub port: Option<u16>,
    /// The password to connect to the server.
    pub password: Option<String>,
    /// Whether or not to use SSL.
    /// Clients will automatically panic if this is enabled without SSL support.
    pub use_ssl: Option<bool>,
    /// The path to the SSL certificate for this server in DER format.
    pub cert_path: Option<String>,
    /// The encoding type used for this connection.
    /// This is typically UTF-8, but could be something else.
    pub encoding: Option<String>,
    /// A list of channels to join on connection.
    pub channels: Option<Vec<String>>,
    /// User modes to set on connect. Example: "+RB -x"
    pub umodes: Option<String>,
    /// The text that'll be sent in response to CTCP USERINFO requests.
    pub user_info: Option<String>,
    /// The text that'll be sent in response to CTCP VERSION requests.
    pub version: Option<String>,
    /// The text that'll be sent in response to CTCP SOURCE requests.
    pub source: Option<String>,
    /// The amount of inactivity in seconds before the client will ping the server.
    pub ping_time: Option<u32>,
    /// The amount of time in seconds for a client to reconnect due to no ping response.
    pub ping_timeout: Option<u32>,
    /// The amount of time in seconds to consider a window for burst messages.
    pub burst_window_length: Option<u32>,
    /// The maximum number of messages that can be sent in a burst window before they'll be delayed.
    /// Messages are automatically delayed until the start of the next window.
    pub max_messages_in_burst: Option<u32>,
    /// Whether the client should use NickServ GHOST to reclaim its primary nickname if it is in
    /// use. This has no effect if `nick_password` is not set.
    pub should_ghost: Option<bool>,
    /// The command(s) that should be sent to NickServ to recover a nickname. The nickname and
    /// password will be appended in that order after the command.
    /// E.g. `["RECOVER", "RELEASE"]` means `RECOVER nick pass` and `RELEASE nick pass` will be sent
    /// in that order.
    pub ghost_sequence: Option<Vec<String>>,
    /// Whether or not to use a fake connection for testing purposes. You probably will never want
    /// to enable this, but it is used in unit testing for the `irc` crate.
    pub use_mock_connection: Option<bool>,
    /// The initial value used by the fake connection for testing. You probably will never need to
    /// set this, but it is used in unit testing for the `irc` crate.
    pub mock_initial_value: Option<String>,

    /// A mapping of channel names to keys for join-on-connect.
    pub channel_keys: Option<HashMap<String, String>>,
    /// A map of additional options to be stored in config.
    pub options: Option<HashMap<String, String>>,
}

impl Config {
    /// Loads a configuration from the desired path. This will use the file extension to detect
    /// which format to parse the file as (json, toml, or yaml). Using each format requires having
    /// its respective crate feature enabled. Only json is available by default.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Config> {
        let mut file = File::open(&path)?;
        let mut data = String::new();
        file.read_to_string(&mut data)?;

        match path.as_ref().extension().and_then(|s| s.to_str()) {
            Some("json") => Config::load_json(&data),
            Some("toml") => Config::load_toml(&data),
            Some("yaml") | Some("yml") => Config::load_yaml(&data),
            Some(ext) => return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("Failed to decode configuration of unknown format {}", ext),
            ).into()),
            None => return Err(Error::new(
                ErrorKind::InvalidInput,
                "Failed to decode configuration of missing or non-unicode format.",
            ).into()),
        }
    }

    #[cfg(feature = "json")]
    fn load_json(data: &str) -> Result<Config> {
        serde_json::from_str(&data[..]).chain_err(|| {
            let e: error::Error = Error::new(
                ErrorKind::InvalidInput,
                "Failed to decode JSON configuration file.",
            ).into();
            e
        })
    }

    #[cfg(not(feature = "json"))]
    fn load_json(_: &str) -> Result<Config> {
        Err(Error::new(
            ErrorKind::InvalidInput,
            "JSON file decoding is disabled.",
        ).into())
    }

    #[cfg(feature = "toml")]
    fn load_toml(data: &str) -> Result<Config> {
        toml::from_str(&data[..]).chain_err(|| {
            let e: error::Error = Error::new(
                ErrorKind::InvalidInput,
                "Failed to decode TOML configuration file.",
            ).into();
            e
        })
    }

    #[cfg(not(feature = "toml"))]
    fn load_toml(_: &str) -> Result<Config> {
        Err(Error::new(
            ErrorKind::InvalidInput,
            "TOML file decoding is disabled.",
        ).into())
    }

    #[cfg(feature = "yaml")]
    fn load_yaml(data: &str) -> Result<Config> {
        serde_yaml::from_str(&data[..]).chain_err(|| {
            let e: error::Error = Error::new(
                ErrorKind::InvalidInput,
                "Failed to decode YAML configuration file.",
            ).into();
            e
        })
    }

    #[cfg(not(feature = "yaml"))]
    fn load_yaml(_: &str) -> Result<Config> {
        Err(Error::new(
            ErrorKind::InvalidInput,
            "YAML file decoding is disabled.",
        ).into())
    }

    /// Saves a configuration to the desired path. This will use the file extension to detect
    /// which format to parse the file as (json, toml, or yaml). Using each format requires having
    /// its respective crate feature enabled. Only json is available by default.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let mut file = File::create(&path)?;
        let data = match path.as_ref().extension().and_then(|s| s.to_str()) {
            Some("json") => self.save_json()?,
            Some("toml") => self.save_toml()?,
            Some("yaml") | Some("yml") => self.save_yaml()?,
            Some(ext) => return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("Failed to encode configuration of unknown format {}", ext),
            ).into()),
            None => return Err(Error::new(
                ErrorKind::InvalidInput,
                "Failed to encode configuration of missing or non-unicode format.",
            ).into()),
        };
        file.write_all(data.as_bytes())?;
        Ok(())
    }

    #[cfg(feature = "json")]
    fn save_json(&self) -> Result<String> {
        serde_json::to_string(self).chain_err(|| {
            let e: error::Error = Error::new(
                ErrorKind::InvalidInput,
                "Failed to encode JSON configuration file.",
            ).into();
            e
        })
    }

    #[cfg(not(feature = "json"))]
    fn save_json(&self) -> Result<String> {
        Err(Error::new(
            ErrorKind::InvalidInput,
            "JSON file encoding is disabled.",
        ).into())
    }

    #[cfg(feature = "toml")]
    fn save_toml(&self) -> Result<String> {
        toml::to_string(self).chain_err(|| {
            let e: error::Error = Error::new(
                ErrorKind::InvalidInput,
                "Failed to encode TOML configuration file.",
            ).into();
            e
        })
    }

    #[cfg(not(feature = "toml"))]
    fn save_toml(&self) -> Result<String> {
        Err(Error::new(
            ErrorKind::InvalidInput,
            "TOML file encoding is disabled.",
        ).into())
    }

    #[cfg(feature = "yaml")]
    fn save_yaml(&self) -> Result<String> {
        serde_yaml::to_string(self).chain_err(|| {
            let e: error::Error = Error::new(
                ErrorKind::InvalidInput,
                "Failed to encode YAML configuration file.",
            ).into();
            e
        })
    }

    #[cfg(not(feature = "yaml"))]
    fn save_yaml(&self) -> Result<String> {
        Err(Error::new(
            ErrorKind::InvalidInput,
            "YAML file encoding is disabled.",
        ).into())
    }

    /// Determines whether or not the nickname provided is the owner of the bot.
    pub fn is_owner(&self, nickname: &str) -> bool {
        self.owners
            .as_ref()
            .map(|o| o.contains(&nickname.to_owned()))
            .unwrap()
    }

    /// Gets the nickname specified in the configuration.
    /// This will panic if not specified.
    pub fn nickname(&self) -> &str {
        self.nickname.as_ref().map(|s| &s[..]).unwrap()
    }

    /// Gets the bot's nickserv password specified in the configuration.
    /// This defaults to an empty string when not specified.
    pub fn nick_password(&self) -> &str {
        self.nick_password.as_ref().map_or("", |s| &s[..])
    }

    /// Gets the alternate nicknames specified in the configuration.
    /// This defaults to an empty vector when not specified.
    pub fn alternate_nicknames(&self) -> Vec<&str> {
        self.alt_nicks.as_ref().map_or(vec![], |v| {
            v.iter().map(|s| &s[..]).collect()
        })
    }


    /// Gets the username specified in the configuration.
    /// This defaults to the user's nickname when not specified.
    pub fn username(&self) -> &str {
        self.username.as_ref().map_or(self.nickname(), |s| &s)
    }

    /// Gets the real name specified in the configuration.
    /// This defaults to the user's nickname when not specified.
    pub fn real_name(&self) -> &str {
        self.realname.as_ref().map_or(self.nickname(), |s| &s)
    }

    /// Gets the address of the server specified in the configuration.
    /// This panics when not specified.
    pub fn server(&self) -> &str {
        self.server.as_ref().map(|s| &s[..]).unwrap()
    }

    /// Gets the port of the server specified in the configuration.
    /// This defaults to 6667 (or 6697 if use_ssl is specified as true) when not specified.
    pub fn port(&self) -> u16 {
        self.port.as_ref().cloned().unwrap_or(if self.use_ssl() {
            6697
        } else {
            6667
        })
    }

    /// Gets the server and port as a `SocketAddr`.
    /// This panics when server is not specified or the address is malformed.
    pub fn socket_addr(&self) -> Result<SocketAddr> {
        format!("{}:{}", self.server(), self.port()).to_socket_addrs()
            .map(|mut i| i.next().unwrap())
            .map_err(|e| e.into())
    }

    /// Gets the server password specified in the configuration.
    /// This defaults to a blank string when not specified.
    pub fn password(&self) -> &str {
        self.password.as_ref().map_or("", |s| &s)
    }

    /// Gets whether or not to use SSL with this connection.
    /// This defaults to false when not specified.
    pub fn use_ssl(&self) -> bool {
        self.use_ssl.as_ref().cloned().unwrap_or(false)
    }

    /// Gets the path to the SSL certificate in DER format if specified.
    pub fn cert_path(&self) -> Option<&str> {
        self.cert_path.as_ref().map(|s| &s[..])
    }

    /// Gets the encoding to use for this connection. This requires the encode feature to work.
    /// This defaults to UTF-8 when not specified.
    pub fn encoding(&self) -> &str {
        self.encoding.as_ref().map_or("UTF-8", |s| &s)
    }

    /// Gets the channels to join upon connection.
    /// This defaults to an empty vector if it's not specified.
    pub fn channels(&self) -> Vec<&str> {
        self.channels.as_ref().map_or(vec![], |v| {
            v.iter().map(|s| &s[..]).collect()
        })
    }


    /// Gets the key for the specified channel if it exists in the configuration.
    pub fn channel_key(&self, chan: &str) -> Option<&str> {
        self.channel_keys.as_ref().and_then(|m| {
            m.get(&chan.to_owned()).map(|s| &s[..])
        })
    }

    /// Gets the user modes to set on connect specified in the configuration.
    /// This defaults to an empty string when not specified.
    pub fn umodes(&self) -> &str {
        self.umodes.as_ref().map_or("", |s| &s[..])
    }

    /// Gets the string to be sent in response to CTCP USERINFO requests.
    /// This defaults to an empty string when not specified.
    pub fn user_info(&self) -> &str {
        self.user_info.as_ref().map_or("", |s| &s[..])
    }

    /// Gets the string to be sent in response to CTCP VERSION requests.
    /// This defaults to `irc:version:env` when not specified.
    /// For example, `irc:0.12.0:Compiled with rustc`
    pub fn version(&self) -> &str {
        self.version.as_ref().map_or(::VERSION_STR, |s| &s)
    }

    /// Gets the string to be sent in response to CTCP SOURCE requests.
    /// This defaults to `https://github.com/aatxe/irc` when not specified.
    pub fn source(&self) -> &str {
        self.source.as_ref().map_or(
            "https://github.com/aatxe/irc",
            |s| &s[..],
        )
    }

    /// Gets the amount of time in seconds for the interval at which the client pings the server.
    /// This defaults to 180 seconds when not specified.
    pub fn ping_time(&self) -> u32 {
        self.ping_time.as_ref().cloned().unwrap_or(180)
    }

    /// Gets the amount of time in seconds for the client to disconnect after not receiving a ping
    /// response.
    /// This defaults to 10 seconds when not specified.
    pub fn ping_timeout(&self) -> u32 {
        self.ping_timeout.as_ref().cloned().unwrap_or(10)
    }

    /// The amount of time in seconds to consider a window for burst messages.
    /// This defaults to 8 seconds when not specified.
    pub fn burst_window_length(&self) -> u32 {
        self.burst_window_length.as_ref().cloned().unwrap_or(8)
    }

    /// The maximum number of messages that can be sent in a burst window before they'll be delayed.
    /// Messages are automatically delayed until the start of the next window.
    /// This defaults to 15 messages when not specified.
    pub fn max_messages_in_burst(&self) -> u32 {
        self.max_messages_in_burst.as_ref().cloned().unwrap_or(15)
    }

    /// Gets whether or not to attempt nickname reclamation using NickServ GHOST.
    /// This defaults to false when not specified.
    pub fn should_ghost(&self) -> bool {
        self.should_ghost.as_ref().cloned().unwrap_or(false)
    }

    /// Gets the NickServ command sequence to recover a nickname.
    /// This defaults to `["GHOST"]` when not specified.
    pub fn ghost_sequence(&self) -> Vec<&str> {
        self.ghost_sequence.as_ref().map_or(vec!["GHOST"], |v| {
            v.iter().map(|s| &s[..]).collect()
        })
    }

    /// Looks up the specified string in the options map.
    /// This uses indexing, and thus panics when the string is not present.
    /// This will also panic if used and there are no options.
    pub fn get_option(&self, option: &str) -> &str {
        self.options
            .as_ref()
            .map(|o| &o[&option.to_owned()][..])
            .unwrap()
    }

    /// Gets whether or not to use a mock connection for testing.
    /// This defaults to false when not specified.
    pub fn use_mock_connection(&self) -> bool {
        self.use_mock_connection.as_ref().cloned().unwrap_or(false)
    }

    /// Gets the initial value for the mock connection.
    /// This defaults to false when not specified.
    /// This has no effect if `use_mock_connection` is not `true`.
    pub fn mock_initial_value(&self) -> &str {
        self.mock_initial_value.as_ref().map_or("", |s| &s)
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;
    use std::default::Default;
    #[cfg(feature = "json")]
    use std::path::Path;

    use super::Config;

    fn test_config() -> Config {
        Config {
            owners: Some(vec![format!("test")]),
            nickname: Some(format!("test")),
            nick_password: None,
            alt_nicks: None,
            username: Some(format!("test")),
            realname: Some(format!("test")),
            password: Some(String::new()),
            umodes: Some(format!("+BR")),
            server: Some(format!("irc.test.net")),
            port: Some(6667),
            use_ssl: Some(false),
            cert_path: None,
            encoding: Some(format!("UTF-8")),
            channels: Some(vec![format!("#test"), format!("#test2")]),
            channel_keys: None,
            user_info: None,
            version: None,
            source: None,
            ping_time: None,
            ping_timeout: None,
            burst_window_length: None,
            max_messages_in_burst: None,
            should_ghost: None,
            ghost_sequence: None,
            options: Some(HashMap::new()),
            use_mock_connection: None,
            mock_initial_value: None,
        }
    }

    #[test]
    #[cfg(feature = "json")]
    fn load() {
        assert_eq!(Config::load(Path::new("client_config.json")).unwrap(), test_config());
    }

    #[test]
    #[cfg(feature = "json")]
    fn load_from_str() {
        assert_eq!(Config::load("client_config.json").unwrap(), test_config());
    }

    #[test]
    #[cfg(feature = "toml")]
    fn load_from_toml() {
        assert_eq!(Config::load("client_config.toml").unwrap(), test_config());
    }

    #[test]
    #[cfg(feature = "yaml")]
    fn load_from_yaml() {
        assert_eq!(Config::load("client_config.yaml").unwrap(), test_config());
    }

    #[test]
    fn is_owner() {
        let cfg = Config {
            owners: Some(vec![format!("test"), format!("test2")]),
            ..Default::default()
        };
        assert!(cfg.is_owner("test"));
        assert!(cfg.is_owner("test2"));
        assert!(!cfg.is_owner("test3"));
    }

    #[test]
    fn get_option() {
        let cfg = Config {
            options: {
                let mut map = HashMap::new();
                map.insert(format!("testing"), format!("test"));
                Some(map)
            },
            ..Default::default()
        };
        assert_eq!(cfg.get_option("testing"), "test");
    }
}

use serde::de::{Deserializer, MapAccess, Visitor};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::io;

#[derive(Debug)]
pub struct Config {
    pub generators: HashMap<String, GeneratorSpec>,
}

// Custom deserializer to reject unknown top-level fields
impl<'de> Deserialize<'de> for Config {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ConfigVisitor;

        impl<'de> Visitor<'de> for ConfigVisitor {
            type Value = Config;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct Config with only 'generators' field")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Config, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut generators = None;
                let allowed_fields: HashSet<&str> = ["generators"].iter().cloned().collect();

                while let Some(key) = map.next_key::<String>()? {
                    if !allowed_fields.contains(key.as_str()) {
                        return Err(serde::de::Error::custom(format!(
                            "unknown field '{}' at top level, only 'generators' is allowed",
                            key
                        )));
                    }
                    if key == "generators" {
                        if generators.is_some() {
                            return Err(serde::de::Error::duplicate_field("generators"));
                        }
                        generators = Some(map.next_value()?);
                    }
                }

                Ok(Config {
                    generators: generators.unwrap_or_default(),
                })
            }
        }

        deserializer.deserialize_map(ConfigVisitor)
    }
}

#[derive(Debug, Deserialize)]
pub struct GeneratorSpec {
    pub model: String,
    pub inputs: HashMap<String, InputSpec>,
    pub prompt: PromptSpec,
    #[serde(default)]
    pub extract: Option<ExtractSpec>,
}

#[derive(Debug, Deserialize)]
pub struct InputSpec {
    pub source: String,
    #[serde(default = "default_true")]
    pub required: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct PromptSpec {
    pub template: String,
}

#[derive(Debug, Deserialize)]
pub struct ExtractSpec {
    #[serde(rename = "type")]
    pub extract_type: String,
    #[serde(default)]
    pub start: Option<String>,
    #[serde(default)]
    pub end: Option<String>,
}

#[derive(Debug)]
pub enum ConfigError {
    Io(io::Error),
    Parse(String),
}

impl From<io::Error> for ConfigError {
    fn from(err: io::Error) -> Self {
        ConfigError::Io(err)
    }
}

impl From<serde_yaml::Error> for ConfigError {
    fn from(err: serde_yaml::Error) -> Self {
        ConfigError::Parse(err.to_string())
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Io(e) => write!(f, "IO error: {}", e),
            ConfigError::Parse(e) => write!(f, "Parse error: {}", e),
        }
    }
}

impl Config {
    pub fn from_file(path: &str) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&content)?;
        Ok(config)
    }
}

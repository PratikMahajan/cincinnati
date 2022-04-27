//! set the pe config
use parking_lot::RwLock;
use std::ops::Deref;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// declare the config struct
#[derive(Debug, Deserialize)]
struct PEConfigYAML {
    pub verbosity: String,
}

/// check the pe config file to set the log level
pub fn refresh_log_level(verbosity: log::Level, log_level: &Arc<RwLock<log::Level>>) -> ! {
    info!("i'm here");
    // Don't wait on the first iteration
    let mut first_iteration = true;
    let path = Path::new("/tmp/config/pe.yaml");

    loop {
        let config_verbosity = match std::fs::read(&path) {
            Ok(yaml) => match serde_yaml::from_slice(&yaml) {
                Ok(value) => {
                    let config: PEConfigYAML = value;
                    log::Level::from_str(&config.verbosity).unwrap_or_else(|e| {
                        warn!(
                            "Failed to parse PE log level: {:?}: {}",
                            &config.verbosity, e
                        );
                        verbosity
                    })
                }
                Err(e) => {
                    warn!("Failed to deserialize pe config file at {:?}: {}", &path, e);
                    verbosity
                }
            },
            Err(e) => {
                if first_iteration {
                    warn!("Couldn't read pe config {:?}: {}", &path, e);
                }
                verbosity
            }
        };
        if log_level.read().deref() != &config_verbosity {
            *log_level.write() = config_verbosity;
            info!("log level set to {:?}", config_verbosity);
        }
        if first_iteration {
            first_iteration = false;
        } else {
            thread::sleep(Duration::new(120, 0));
        }
    }
}

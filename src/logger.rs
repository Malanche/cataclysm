//! Logger to be removed, based on Borntyping's Simple Logger <https://github.com/borntyping/rust-simple_logger>

use chrono::Local;
use colored::*;
use log::{Level, LevelFilter, Log, Metadata, Record, SetLoggerError};

pub struct SimpleLogger {
    default_level: LevelFilter,
    module_levels: Vec<(String, LevelFilter)>,
}

impl SimpleLogger {
    pub fn new() -> SimpleLogger {
        SimpleLogger {
            default_level: LevelFilter::Trace,
            module_levels: Vec::new(),
        }
    }

    pub fn from_env() -> SimpleLogger {
        let level = match std::env::var("RUST_LOG") {
            Ok(x) => match x.to_lowercase().as_str() {
                "trace" => log::LevelFilter::Trace,
                "debug" => log::LevelFilter::Debug,
                "info" => log::LevelFilter::Info,
                "warn" => log::LevelFilter::Warn,
                _ => log::LevelFilter::Error,
            },
            _ => log::LevelFilter::Error,
        };

        SimpleLogger::new().with_level(level)
    }

    pub fn with_level(mut self, level: LevelFilter) -> SimpleLogger {
        self.default_level = level;
        self
    }

    pub fn with_module_level(mut self, target: &str, level: LevelFilter) -> SimpleLogger {
        self.module_levels.push((target.to_string(), level));

        /* Normally this is only called in `init` to avoid redundancy, but we can't initialize the logger in tests */
        #[cfg(test)]
        self.module_levels
            .sort_by_key(|(name, _level)| name.len().wrapping_neg());

        self
    }

    pub fn init(mut self) -> Result<(), SetLoggerError> {
        self.module_levels
            .sort_by_key(|(name, _level)| name.len().wrapping_neg());
        let max_level = self
            .module_levels
            .iter()
            .map(|(_name, level)| level)
            .copied()
            .max();
        let max_level = max_level
            .map(|lvl| lvl.max(self.default_level))
            .unwrap_or(self.default_level);
        log::set_max_level(max_level);
        log::set_boxed_logger(Box::new(self))?;
        Ok(())
    }
}

impl Default for SimpleLogger {
    fn default() -> Self {
        SimpleLogger::new()
    }
}

impl Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        &metadata.level().to_level_filter()
            <= self
                .module_levels
                .iter()
                .find(|(name, _level)| metadata.target().starts_with(name))
                .map(|(_name, level)| level)
                .unwrap_or(&self.default_level)
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let level_string = {
                match record.level() {
                    Level::Error => record.level().to_string().red(),
                    Level::Warn => record.level().to_string().yellow(),
                    Level::Info => record.level().to_string().cyan(),
                    Level::Debug => record.level().to_string().purple(),
                    Level::Trace => record.level().to_string().normal(),
                }
            };
            let target = if !record.target().is_empty() {
                record.target()
            } else {
                record.module_path().unwrap_or_default()
            };
            println!(
                "{} {:<5} [{}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S,%3f"),
                level_string,
                target,
                record.args()
            );
        }
    }

    fn flush(&self) {}
}
use std::{str::FromStr, time::SystemTime};

use log::Level;

pub fn setup_logger() {
    const ERROR_ONLY_TARGETS: &[&str] = &["aws_config", "tracing", "rocket", "sqlx"];
    let log_level: ::log::LevelFilter =
        ::log::LevelFilter::from_str(&std::env::var("LOG_LEVEL").unwrap_or("INFO".to_string()))
            .expect("Log level environment variable not set to a valid value.");

    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                humantime::format_rfc3339_seconds(SystemTime::now()),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log_level)
        .filter(|r| {
            let is_error_only_target: bool = ERROR_ONLY_TARGETS
                .iter()
                .any(|target| r.target().contains(target));

            !is_error_only_target || r.level() <= Level::Error
        })
        .chain(std::io::stderr())
        .apply()
        .expect("Couldn't start log dispatcher.");
}

/*
 * Copyright 2026-present Viktor Popp
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
use log::Level;
use std::{error::Error, sync::Mutex};

static LOGGER: Logger = Logger;
static LOG_LEVEL: Mutex<Level> = Mutex::new(Level::Debug);

const fn level_to_color(level: &Level) -> &str {
    match level {
        Level::Error => "\x1b[1;91m",
        Level::Warn => "\x1b[1;93m",
        Level::Info => "\x1b[1;92m",
        Level::Debug => "\x1b[1;94m",
        Level::Trace => "\x1b[1;90m",
    }
}

struct Logger;

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= *(LOG_LEVEL.lock().unwrap())
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        println!(
            "{}{:>5}\x1b[0m {}",
            level_to_color(&record.level()),
            record.level().as_str().to_lowercase(),
            record.args()
        );
    }

    fn flush(&self) {}
}

pub fn init() -> Result<(), Box<dyn Error>> {
    log::set_logger(&LOGGER).map_err(|e| Box::new(e) as Box<dyn Error>)?;
    log::set_max_level(log::LevelFilter::Trace);
    Ok(())
}

pub fn set_level(level: Level) -> Result<(), Box<dyn Error>> {
    let mut log_level = LOG_LEVEL.lock()?;
    *log_level = level;
    Ok(())
}

#[allow(unused)]
pub fn level() -> Result<Level, Box<dyn Error>> {
    let log_level = LOG_LEVEL.lock()?;
    Ok(*log_level)
}

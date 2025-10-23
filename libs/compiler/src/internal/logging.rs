use std::fmt::Write as _;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, RwLock};

use log::{Level, LevelFilter, Log, Metadata, Record};
use napi::bindgen_prelude::*;
#[cfg(not(test))]
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
#[cfg(not(test))]
use napi::{JsFunction, JsObject, JsUnknown};
use once_cell::sync::OnceCell;

use crate::internal::errors::Result;

/// Shared logging level exposed to both Rust and JavaScript callers.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum LoggingLevel {
  #[default]
  Silent,
  Error,
  Warn,
  Info,
}

impl LoggingLevel {
  const fn to_index(self) -> u8 {
    match self {
      LoggingLevel::Silent => 0,
      LoggingLevel::Error => 1,
      LoggingLevel::Warn => 2,
      LoggingLevel::Info => 3,
    }
  }

  const fn from_index(value: u8) -> Self {
    match value {
      0 => LoggingLevel::Silent,
      1 => LoggingLevel::Error,
      2 => LoggingLevel::Warn,
      _ => LoggingLevel::Info,
    }
  }

  pub fn as_str(self) -> &'static str {
    match self {
      LoggingLevel::Silent => "silent",
      LoggingLevel::Error => "error",
      LoggingLevel::Warn => "warn",
      LoggingLevel::Info => "info",
    }
  }
}

impl From<LoggingLevel> for LevelFilter {
  fn from(level: LoggingLevel) -> Self {
    match level {
      LoggingLevel::Silent => LevelFilter::Off,
      LoggingLevel::Error => LevelFilter::Error,
      LoggingLevel::Warn => LevelFilter::Warn,
      LoggingLevel::Info => LevelFilter::Info,
    }
  }
}

impl From<Level> for LoggingLevel {
  fn from(level: Level) -> Self {
    match level {
      Level::Error => LoggingLevel::Error,
      Level::Warn => LoggingLevel::Warn,
      Level::Info | Level::Debug | Level::Trace => LoggingLevel::Info,
    }
  }
}

/// Install the shared logger when invoked from Rust-only entry points.
pub fn ensure_rust_logger(level: LoggingLevel) -> Result<()> {
  let state = install_logger()?;
  state.ensure_stdout_backend();
  state.update_level(level);
  Ok(())
}

/// Install the shared logger when invoked from a JavaScript (N-API) context.
#[cfg(test)]
pub fn ensure_napi_logger(_: &Env, _: LoggingLevel) -> napi::Result<()> {
  Ok(())
}

#[cfg(not(test))]
pub fn ensure_napi_logger(env: &Env, level: LoggingLevel) -> napi::Result<()> {
  let state = install_logger().map_err(napi::Error::from)?;
  let tsfn = create_threadsafe_logger(env)?;
  state.set_node_backend(tsfn);
  state.update_level(level);
  Ok(())
}

/// Update the active logging level in place.
pub fn update_level(level: LoggingLevel) {
  if let Some(state) = LOGGER.get() {
    state.update_level(level);
  } else {
    log::set_max_level(LevelFilter::from(level));
  }
}

#[cfg(not(test))]
#[derive(Clone)]
struct LogInvocation {
  level: LoggingLevel,
  line: String,
}

enum LoggerBackend {
  Stdout,
  #[cfg(not(test))]
  Node(ThreadsafeFunction<LogInvocation>),
}

struct LoggerState {
  backend: RwLock<Option<LoggerBackend>>,
  level: AtomicU8,
}

impl LoggerState {
  fn new() -> Self {
    Self {
      backend: RwLock::new(None),
      level: AtomicU8::new(LoggingLevel::Info.to_index()),
    }
  }

  fn ensure_stdout_backend(&self) {
    let mut backend = self.backend.write().expect("logger backend lock poisoned");
    if backend.is_none() {
      *backend = Some(LoggerBackend::Stdout);
    }
  }

  #[cfg(not(test))]
  fn set_node_backend(&self, tsfn: ThreadsafeFunction<LogInvocation>) {
    let mut backend = self.backend.write().expect("logger backend lock poisoned");
    *backend = Some(LoggerBackend::Node(tsfn));
  }

  fn update_level(&self, level: LoggingLevel) {
    self.level.store(level.to_index(), Ordering::Release);
    log::set_max_level(LevelFilter::from(level));
  }

  fn active_level(&self) -> LoggingLevel {
    LoggingLevel::from_index(self.level.load(Ordering::Acquire))
  }

  fn backend_guard(&self) -> std::sync::RwLockReadGuard<'_, Option<LoggerBackend>> {
    self.backend.read().expect("logger backend lock poisoned")
  }
}

struct ConsoleLogger {
  state: Arc<LoggerState>,
}

impl ConsoleLogger {
  fn new(state: Arc<LoggerState>) -> Self {
    Self { state }
  }
}

impl Log for ConsoleLogger {
  fn enabled(&self, metadata: &Metadata<'_>) -> bool {
    metadata.level() <= LevelFilter::from(self.state.active_level())
  }

  fn log(&self, record: &Record<'_>) {
    if !self.enabled(record.metadata()) {
      return;
    }

    let level = LoggingLevel::from(record.level());
    let mut line = String::new();
    let _ = write!(&mut line, "[{}]", level.as_str().to_uppercase());

    let target = record.target();
    if !target.is_empty() {
      let _ = write!(&mut line, " {target}");
    }

    if let (Some(file), Some(line_no)) = (record.file(), record.line()) {
      let _ = write!(&mut line, " ({file}:{line_no})");
    }

    let _ = write!(&mut line, " - {}", record.args());

    let backend_guard = self.state.backend_guard();
    match backend_guard.as_ref() {
      Some(LoggerBackend::Stdout) => dispatch_stdout(level, &line),
      #[cfg(not(test))]
      Some(LoggerBackend::Node(tsfn)) => {
        let _ = tsfn.call(
          Ok(LogInvocation {
            level,
            line: line.clone(),
          }),
          ThreadsafeFunctionCallMode::NonBlocking,
        );
      }
      None => dispatch_stdout(level, &line),
    }
  }

  fn flush(&self) {}
}

fn dispatch_stdout(level: LoggingLevel, line: &str) {
  match level {
    LoggingLevel::Error | LoggingLevel::Warn => eprintln!("{line}"),
    LoggingLevel::Silent => {}
    LoggingLevel::Info => println!("{line}"),
  }
}

fn install_logger() -> Result<Arc<LoggerState>> {
  let state = LOGGER.get_or_init(|| Arc::new(LoggerState::new())).clone();

  if log::set_boxed_logger(Box::new(ConsoleLogger::new(state.clone()))).is_ok() {
    log::set_max_level(LevelFilter::from(LoggingLevel::Info));
  }

  Ok(state)
}

#[cfg(not(test))]
fn create_threadsafe_logger(env: &Env) -> napi::Result<ThreadsafeFunction<LogInvocation>> {
  let console: JsObject = env.get_global()?.get_named_property("console")?;
  let log_fn: JsFunction = console.get_named_property("log")?;
  env.create_threadsafe_function::<LogInvocation, JsUnknown, _>(&log_fn, 0, |ctx| {
    let console: JsObject = ctx.env.get_global()?.get_named_property("console")?;
    let method = match ctx.value.level {
      LoggingLevel::Error => "error",
      LoggingLevel::Warn => "warn",
      LoggingLevel::Silent | LoggingLevel::Info => "log",
    };
    let js_fn: JsFunction = console
      .get_named_property(method)
      .or_else(|_| console.get_named_property("log"))?;
    let message = ctx.env.create_string(&ctx.value.line)?;
    js_fn.call(Some(&console), &[message.into_unknown()])?;
    Ok(Vec::new())
  })
}

static LOGGER: OnceCell<Arc<LoggerState>> = OnceCell::new();

#[cfg(test)]
mod tests {
  use super::*;
  use log::LevelFilter;

  #[test]
  fn logging_level_index_roundtrip() {
    for level in [
      LoggingLevel::Silent,
      LoggingLevel::Error,
      LoggingLevel::Warn,
      LoggingLevel::Info,
    ] {
      assert_eq!(LoggingLevel::from_index(level.to_index()), level);
    }
  }

  #[test]
  fn ensure_logger_controls_max_level() {
    update_level(LoggingLevel::Silent);
    assert_eq!(log::max_level(), LevelFilter::Off);

    ensure_rust_logger(LoggingLevel::Info).expect("install info logger");
    assert_eq!(log::max_level(), LevelFilter::Info);

    update_level(LoggingLevel::Error);
    assert_eq!(log::max_level(), LevelFilter::Error);

    ensure_rust_logger(LoggingLevel::Warn).expect("update logger level");
    assert_eq!(log::max_level(), LevelFilter::Warn);
  }
}

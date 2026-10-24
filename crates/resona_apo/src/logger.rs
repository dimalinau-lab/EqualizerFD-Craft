//! Lightweight, Non-Panicking Diagnostic Logger for ResonaAPO
//! Writes events to d:\testik\ResonaAPO.log

use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;
use std::time::SystemTime;

static LOG_MUTEX: Mutex<()> = Mutex::new(());

pub fn log(msg: &str) {
    let _lock = match LOG_MUTEX.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    let log_line = format!("[ResonaAPO +{}ms] {}\r\n", now % 100_000, msg);

    if let Ok(mut f) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(r"d:\testik\ResonaAPO.log")
    {
        let _ = f.write_all(log_line.as_bytes());
        let _ = f.flush();
    }
}

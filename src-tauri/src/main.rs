// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    std::panic::set_hook(Box::new(|info| {
        log::error!("ORBIT process panicked: {info}");
        if let Ok(app_data) = std::env::var("APPDATA") {
            let log_dir = std::path::PathBuf::from(app_data).join("com.orbit.island").join("logs");
            if std::fs::create_dir_all(&log_dir).is_ok() {
                let crash_file = log_dir.join("crash.log");
                let location = info
                    .location()
                    .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
                    .unwrap_or_else(|| "unknown".to_string());
                let message = if let Some(s) = info.payload().downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = info.payload().downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Non-string panic payload".to_string()
                };
                let log_entry = format!(
                    "[{:?}] Process panic at {location}: {message}\n",
                    std::time::SystemTime::now()
                );
                use std::io::Write;
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(crash_file)
                {
                    let _ = f.write_all(log_entry.as_bytes());
                }
            }
        }
    }));
    orbit_lib::run();
}

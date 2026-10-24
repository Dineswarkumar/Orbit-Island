use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub const MAX_LOG_SIZE_BYTES: u64 = 2 * 1024 * 1024; // 2 MB

static LOG_LOCK: Mutex<()> = Mutex::new(());

/// Resolves the default diagnostic log file path:
/// `%APPDATA%\com.orbit.island\logs\orbit.log`
pub fn get_default_log_path() -> PathBuf {
    if let Ok(app_data) = std::env::var("APPDATA") {
        PathBuf::from(app_data)
            .join("com.orbit.island")
            .join("logs")
            .join("orbit.log")
    } else if let Ok(user_profile) = std::env::var("USERPROFILE") {
        PathBuf::from(user_profile)
            .join("AppData")
            .join("Roaming")
            .join("com.orbit.island")
            .join("logs")
            .join("orbit.log")
    } else {
        std::env::temp_dir()
            .join("com.orbit.island")
            .join("logs")
            .join("orbit.log")
    }
}

/// Helper function to convert epoch seconds to YYYY-MM-DD HH:MM:SS format
/// without external dependencies (pure civil time calculation).
pub fn civil_from_timestamp(secs: u64) -> String {
    let sec = (secs % 60) as u32;
    let total_mins = secs / 60;
    let min = (total_mins % 60) as u32;
    let total_hours = total_mins / 60;
    let hour = (total_hours % 24) as u32;
    let mut days = (total_hours / 24) as i64;

    // Howard Hinnant algorithm for civil date calculation from day count
    days += 719468;
    let era = if days >= 0 { days } else { days - 146096 } / 146097;
    let doe = (days - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", y, m, d, hour, min, sec)
}

/// Formats the current local or system timestamp as `YYYY-MM-DD HH:MM:SS`.
pub fn format_current_timestamp() -> String {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::Foundation::SYSTEMTIME;
        use windows_sys::Win32::System::SystemInformation::GetLocalTime;
        let mut st: SYSTEMTIME = unsafe { std::mem::zeroed() };
        unsafe { GetLocalTime(&mut st) };
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        civil_from_timestamp(secs)
    }
}

/// Helper function that checks log size, rotates `orbit.log` -> `orbit.log.1` if
/// `max_bytes` is exceeded, and appends the line.
/// Extracted so unit tests can test rotation in an isolated directory.
pub fn write_and_rotate_log(log_path: &Path, max_bytes: u64, line: &str) -> io::Result<()> {
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Check if the current log exceeds max_bytes before appending
    if log_path.exists() {
        let current_size = match fs::metadata(log_path) {
            Ok(meta) => meta.len(),
            Err(_) => 0,
        };

        if current_size >= max_bytes {
            let mut rotated_name = log_path.file_name().unwrap_or_default().to_os_string();
            rotated_name.push(".1");
            let rotated_path = log_path.with_file_name(rotated_name);

            if rotated_path.exists() {
                let _ = fs::remove_file(&rotated_path);
            }
            let _ = fs::rename(log_path, &rotated_path);
        }
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;

    let formatted_line = if line.ends_with('\n') {
        line.to_string()
    } else {
        format!("{line}\n")
    };

    file.write_all(formatted_line.as_bytes())?;
    file.flush()?;

    Ok(())
}

/// Formats and persists a diagnostic event to `%APPDATA%\com.orbit.island\logs\orbit.log`.
/// Format: `[YYYY-MM-DD HH:MM:SS] [LEVEL] [MODULE] message\n`
/// Enforces the privacy rule: notification titles and bodies must never be logged.
pub fn log_event(level: &str, module: &str, message: &str) {
    let _guard = LOG_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // Privacy guardrail: Never log notification title or body text
    let sanitized_msg = if module.eq_ignore_ascii_case("notifications")
        || module.eq_ignore_ascii_case("notification")
    {
        if message.contains("title:") || message.contains("body:") {
            "[REDACTED NOTIFICATION CONTENT]"
        } else {
            message
        }
    } else {
        message
    };

    let timestamp = format_current_timestamp();
    let line = format!("[{timestamp}] [{}] [{module}] {sanitized_msg}", level.to_uppercase());
    let log_path = get_default_log_path();
    let _ = write_and_rotate_log(&log_path, MAX_LOG_SIZE_BYTES, &line);
}

/// Initializes the rotating diagnostic logger and records startup event.
pub fn init_logger() {
    let log_path = get_default_log_path();
    if let Some(parent) = log_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    std::panic::set_hook(Box::new(|info| {
        let msg = format!("Panic: {info}");
        log_event("FATAL", "panic", &msg);
    }));

    log_event("INFO", "system", "ORBIT diagnostic logger initialized");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn logger_rotates_when_exceeding_max_bytes() {
        let temp_dir = std::env::temp_dir().join(format!(
            "orbit_logger_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::create_dir_all(&temp_dir);
        let log_path = temp_dir.join("orbit.log");
        let rotated_path = temp_dir.join("orbit.log.1");

        // Use a 40-byte threshold for testing
        let max_bytes = 40;

        // 1. Initial write: 45 chars + newline = 46 bytes (> 40 max_bytes)
        let entry1 = "First log line entry that exceeds forty bytes";
        write_and_rotate_log(&log_path, max_bytes, entry1).expect("First write should succeed");

        assert!(log_path.exists(), "orbit.log must exist after first write");
        assert!(!rotated_path.exists(), "orbit.log.1 must not exist yet");

        let content1 = fs::read_to_string(&log_path).expect("Read log");
        assert!(content1.contains("First log line"));

        // 2. Second write: since current_size (46) >= max_bytes (40), rotation triggers
        let entry2 = "Trigger rotation line - gen 2";
        write_and_rotate_log(&log_path, max_bytes, entry2).expect("Second write should succeed");

        assert!(rotated_path.exists(), "orbit.log.1 must exist after rotation");
        assert!(log_path.exists(), "New orbit.log must exist after rotation");

        let rotated_content = fs::read_to_string(&rotated_path).expect("Read rotated log");
        let current_content = fs::read_to_string(&log_path).expect("Read current log");

        assert!(
            rotated_content.contains("First log line"),
            "Rotated log must contain generation 1 entry"
        );
        assert!(
            current_content.contains("Trigger rotation line - gen 2"),
            "Current log must contain generation 2 entry"
        );
        assert!(
            !current_content.contains("First log line"),
            "Current log must not contain rotated content"
        );

        // 3. Third write: current_size of gen 2 (30 chars) + extra line to exceed 40 bytes
        let _ = write_and_rotate_log(&log_path, max_bytes, "Additional bytes to exceed max_bytes");
        let entry3 = "Third generation line";
        write_and_rotate_log(&log_path, max_bytes, entry3)
            .expect("Third write should succeed and replace .1");

        let updated_rotated = fs::read_to_string(&rotated_path).expect("Read updated rotated log");
        let updated_current = fs::read_to_string(&log_path).expect("Read updated current log");

        assert!(
            updated_rotated.contains("Trigger rotation line - gen 2"),
            "Rotated log must now contain generation 2"
        );
        assert!(
            updated_current.contains("Third generation line"),
            "Current log must contain generation 3"
        );

        // Clean up temp test directory
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn civil_date_calculation_is_accurate() {
        // 2026-10-16 12:00:00 UTC = 1792152000 secs approximately
        // 1700000000 = 2023-11-14 22:13:20 UTC
        let formatted = civil_from_timestamp(1700000000);
        assert_eq!(formatted, "2023-11-14 22:13:20");
    }
}

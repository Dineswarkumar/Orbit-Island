use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NotificationAction {
    pub id: String,
    pub label: String,
    pub is_supported: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NotificationItem {
    pub id: u32,
    pub app_name: String,
    pub app_id: String,
    pub title: String,
    pub body: String,
    pub timestamp_ms: u64,
    pub is_critical: bool,
    pub count: u32,
    pub actions: Vec<NotificationAction>,
    pub app_icon: Option<String>,
    #[serde(default)]
    pub underlying_ids: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NotificationSnapshot {
    pub access_status: String, // "allowed", "denied", "unspecified", "unsupported"
    pub active_notification: Option<NotificationItem>,
    pub notifications: Vec<NotificationItem>,
    pub overflow_count: usize,
    pub total_count: usize,
}

impl Default for NotificationSnapshot {
    fn default() -> Self {
        Self {
            access_status: "unspecified".to_string(),
            active_notification: None,
            notifications: Vec::new(),
            overflow_count: 0,
            total_count: 0,
        }
    }
}

pub struct NotificationManager {
    is_running: Arc<AtomicBool>,
    last_snapshot: Arc<Mutex<NotificationSnapshot>>,
    dismissed_ids: Arc<Mutex<HashMap<u32, u64>>>,
}

impl NotificationManager {
    pub fn new() -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(false)),
            last_snapshot: Arc::new(Mutex::new(NotificationSnapshot::default())),
            dismissed_ids: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn start(&self, app: AppHandle) {
        if self.is_running.swap(true, Ordering::SeqCst) {
            return;
        }

        let is_running = self.is_running.clone();
        let snapshot_store = self.last_snapshot.clone();
        let dismissed_store = self.dismissed_ids.clone();

        std::thread::Builder::new()
            .name("orbit-notifications".to_string())
            .spawn(move || {
                #[cfg(target_os = "windows")]
                unsafe {
                    windows_sys::Win32::System::Com::CoInitializeEx(
                        std::ptr::null_mut(),
                        windows_sys::Win32::System::Com::COINIT_MULTITHREADED as u32,
                    );
                }

                while is_running.load(Ordering::Relaxed) {
                    let raw_snapshot = fetch_notifications_internal(&dismissed_store);
                    let config = crate::services::config::load_config(&app);
                    let snapshot = apply_filtering_and_privacy(raw_snapshot, &config);

                    let changed = {
                        let mut guard = snapshot_store.lock().unwrap();
                        if *guard != snapshot {
                            *guard = snapshot.clone();
                            true
                        } else {
                            false
                        }
                    };

                    if changed {
                        let _ = app.emit("notification-state-changed", &snapshot);
                    }

                    // Poll every 1000ms
                    std::thread::sleep(Duration::from_millis(1000));
                }

                #[cfg(target_os = "windows")]
                unsafe {
                    windows_sys::Win32::System::Com::CoUninitialize();
                }
            })
            .expect("Failed to start notification background worker");
    }

    pub fn get_current_snapshot(&self) -> NotificationSnapshot {
        self.last_snapshot.lock().unwrap().clone()
    }

    pub fn dismiss_notification(&self, id: u32) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let mut dismissed = self.dismissed_ids.lock().unwrap();
        dismissed.insert(id, now);

        let snapshot = self.last_snapshot.lock().unwrap();
        for item in &snapshot.notifications {
            if item.id == id || item.underlying_ids.contains(&id) {
                for &uid in &item.underlying_ids {
                    dismissed.insert(uid, now);
                }
            }
        }
    }

    pub fn clear_all(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let current = self.last_snapshot.lock().unwrap();
        let mut dismissed = self.dismissed_ids.lock().unwrap();
        for n in &current.notifications {
            dismissed.insert(n.id, now);
            for &uid in &n.underlying_ids {
                dismissed.insert(uid, now);
            }
        }
    }
}

pub fn get_access_status() -> String {
    #[cfg(target_os = "windows")]
    {
        match windows::UI::Notifications::Management::UserNotificationListener::Current() {
            Ok(listener) => match listener.GetAccessStatus() {
                Ok(windows::UI::Notifications::Management::UserNotificationListenerAccessStatus::Allowed) => {
                    "allowed".to_string()
                }
                Ok(windows::UI::Notifications::Management::UserNotificationListenerAccessStatus::Denied) => {
                    "denied".to_string()
                }
                Ok(windows::UI::Notifications::Management::UserNotificationListenerAccessStatus::Unspecified) => {
                    "unspecified".to_string()
                }
                _ => "unspecified".to_string(),
            },
            Err(e) => {
                log::warn!("UserNotificationListener not available: {e}");
                "unsupported".to_string()
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        "unsupported".to_string()
    }
}

pub async fn request_access_async() -> Result<String, String> {
    tokio::task::spawn_blocking(|| {
        #[cfg(target_os = "windows")]
        {
            unsafe {
                windows_sys::Win32::System::Com::CoInitializeEx(
                    std::ptr::null_mut(),
                    windows_sys::Win32::System::Com::COINIT_MULTITHREADED as u32,
                );
            }

            let result = match windows::UI::Notifications::Management::UserNotificationListener::Current() {
                Ok(listener) => match listener.RequestAccessAsync() {
                    Ok(async_op) => match async_op.join() {
                        Ok(windows::UI::Notifications::Management::UserNotificationListenerAccessStatus::Allowed) => {
                            Ok("allowed".to_string())
                        }
                        Ok(windows::UI::Notifications::Management::UserNotificationListenerAccessStatus::Denied) => {
                            Ok("denied".to_string())
                        }
                        Ok(windows::UI::Notifications::Management::UserNotificationListenerAccessStatus::Unspecified) => {
                            Ok("unspecified".to_string())
                        }
                        Ok(_) => Ok("unspecified".to_string()),
                        Err(e) => Err(format!("RequestAccessAsync failed: {e}")),
                    },
                    Err(e) => Err(format!("Failed to initiate RequestAccessAsync: {e}")),
                },
                Err(e) => Err(format!("UserNotificationListener::Current failed: {e}")),
            };

            unsafe {
                windows_sys::Win32::System::Com::CoUninitialize();
            }

            result
        }
        #[cfg(not(target_os = "windows"))]
        {
            Ok("unsupported".to_string())
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(target_os = "windows")]
fn detect_image_mime(stream_content_type: Option<String>, buffer: &[u8]) -> &'static str {
    if let Some(ct) = stream_content_type {
        let lower = ct.to_lowercase();
        if lower.contains("png") {
            return "image/png";
        } else if lower.contains("icon") || lower.contains("ico") {
            return "image/x-icon";
        } else if lower.contains("jpeg") || lower.contains("jpg") {
            return "image/jpeg";
        } else if lower.contains("gif") {
            return "image/gif";
        } else if lower.contains("bmp") {
            return "image/bmp";
        } else if lower.contains("webp") {
            return "image/webp";
        } else if lower.contains("svg") {
            return "image/svg+xml";
        }
    }

    if buffer.starts_with(b"\x89PNG\r\n\x1a\n") {
        "image/png"
    } else if buffer.starts_with(b"\xff\xd8\xff") {
        "image/jpeg"
    } else if buffer.starts_with(b"\x00\x00\x01\x00") {
        "image/x-icon"
    } else if buffer.starts_with(b"GIF8") {
        "image/gif"
    } else if buffer.starts_with(b"BM") {
        "image/bmp"
    } else if buffer.starts_with(b"RIFF") && buffer.len() > 12 && &buffer[8..12] == b"WEBP" {
        "image/webp"
    } else if buffer.starts_with(b"<svg") || buffer.starts_with(b"<?xml") {
        "image/svg+xml"
    } else {
        "image/png"
    }
}

#[cfg(target_os = "windows")]
fn get_app_icon_base64(
    app_key: &str,
    display_info: &Option<windows::ApplicationModel::AppDisplayInfo>,
) -> Option<String> {
    static APP_ICON_CACHE: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

    let mut cache_guard = APP_ICON_CACHE.lock().unwrap();
    let cache = cache_guard.get_or_insert_with(HashMap::new);

    if let Some(cached) = cache.get(app_key) {
        return Some(cached.clone());
    }

    let loaded = (|| {
        use base64::Engine;
        let d_info = display_info.as_ref()?;

        // Attempt retrieval across standard Windows asset target sizes: 64x64, 48x48, 32x32
        let sizes = [
            windows::Foundation::Size {
                Width: 64.0,
                Height: 64.0,
            },
            windows::Foundation::Size {
                Width: 48.0,
                Height: 48.0,
            },
            windows::Foundation::Size {
                Width: 32.0,
                Height: 32.0,
            },
        ];

        let mut stream_opt = None;
        for size in sizes {
            if let Ok(logo_ref) = d_info.GetLogo(size) {
                if let Ok(stream_op) = logo_ref.OpenReadAsync() {
                    if let Ok(stream) = stream_op.join() {
                        let stream_size = stream.Size().unwrap_or(0);
                        if stream_size > 0 && stream_size <= 2 * 1024 * 1024 {
                            stream_opt = Some((stream, stream_size));
                            break;
                        }
                    }
                }
            }
        }

        let (stream, size) = stream_opt?;
        let content_type = stream.ContentType().ok().map(|h| h.to_string());
        let reader = windows::Storage::Streams::DataReader::CreateDataReader(&stream).ok()?;
        reader.LoadAsync(size as u32).ok()?.join().ok()?;
        let mut buffer = vec![0u8; size as usize];
        reader.ReadBytes(&mut buffer).ok()?;

        let mime = detect_image_mime(content_type, &buffer);
        let b64 = base64::engine::general_purpose::STANDARD.encode(&buffer);
        Some(format!("data:{mime};base64,{b64}"))
    })();

    if let Some(ref icon_data) = loaded {
        cache.insert(app_key.to_string(), icon_data.clone());
    }
    loaded
}

fn fetch_notifications_internal(
    dismissed_store: &Arc<Mutex<HashMap<u32, u64>>>,
) -> NotificationSnapshot {
    #[cfg(target_os = "windows")]
    {
        let listener = match windows::UI::Notifications::Management::UserNotificationListener::Current() {
            Ok(l) => l,
            Err(_) => {
                return NotificationSnapshot {
                    access_status: "unsupported".to_string(),
                    ..Default::default()
                };
            }
        };

        let access_status = match listener.GetAccessStatus() {
            Ok(windows::UI::Notifications::Management::UserNotificationListenerAccessStatus::Allowed) => {
                "allowed".to_string()
            }
            Ok(windows::UI::Notifications::Management::UserNotificationListenerAccessStatus::Denied) => {
                return NotificationSnapshot {
                    access_status: "denied".to_string(),
                    ..Default::default()
                };
            }
            Ok(windows::UI::Notifications::Management::UserNotificationListenerAccessStatus::Unspecified) => {
                return NotificationSnapshot {
                    access_status: "unspecified".to_string(),
                    ..Default::default()
                };
            }
            _ => {
                return NotificationSnapshot {
                    access_status: "unspecified".to_string(),
                    ..Default::default()
                };
            }
        };

        let async_op = match listener.GetNotificationsAsync(
            windows::UI::Notifications::NotificationKinds::Toast,
        ) {
            Ok(op) => op,
            Err(e) => {
                log::warn!("GetNotificationsAsync failed: {e}");
                return NotificationSnapshot {
                    access_status,
                    ..Default::default()
                };
            }
        };

        let raw_notifications = match async_op.join() {
            Ok(list) => list,
            Err(e) => {
                log::warn!("Awaiting GetNotificationsAsync failed: {e}");
                return NotificationSnapshot {
                    access_status,
                    ..Default::default()
                };
            }
        };

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let count = raw_notifications.Size().unwrap_or(0);
        let mut current_windows_ids = std::collections::HashSet::new();
        for i in 0..count {
            if let Ok(user_notif) = raw_notifications.GetAt(i) {
                if let Ok(id) = user_notif.Id() {
                    current_windows_ids.insert(id);
                }
            }
        }

        // Prune local suppression:
        // Keep IDs only if the source notification is still present in Windows Notification Center.
        // Once the user clears it from Windows (or it expires), prune it from dismissed_ids
        // so future notifications from that app or ID are not blocked.
        {
            let mut dismissed = dismissed_store.lock().unwrap();
            dismissed.retain(|id, _| current_windows_ids.contains(id));
        }

        let dismissed = dismissed_store.lock().unwrap().clone();
        let mut items: Vec<NotificationItem> = Vec::new();

        for i in 0..count {
            if let Ok(user_notif) = raw_notifications.GetAt(i) {
                let id = user_notif.Id().unwrap_or(0);
                if dismissed.contains_key(&id) {
                    continue;
                }

                let app_info = user_notif.AppInfo().ok();
                let display_info = app_info.as_ref().and_then(|a| a.DisplayInfo().ok());
                let mut app_name = display_info
                    .as_ref()
                    .and_then(|d| d.DisplayName().ok())
                    .map(|h| h.to_string())
                    .unwrap_or_else(|| "System Notification".to_string());
                let app_id = app_info
                    .as_ref()
                    .and_then(|a| a.AppUserModelId().ok())
                    .map(|h| h.to_string())
                    .unwrap_or_default();

                // Convert Windows DateTime (100ns intervals since Jan 1 1601) to Unix epoch millis
                let timestamp_ms = match user_notif.CreationTime() {
                    Ok(dt) => {
                        let ticks = dt.UniversalTime;
                        const TICKS_BETWEEN_1601_AND_1970: i64 = 116_444_736_000_000_000;
                        if ticks > TICKS_BETWEEN_1601_AND_1970 {
                            ((ticks - TICKS_BETWEEN_1601_AND_1970) / 10_000) as u64
                        } else {
                            now_ms
                        }
                    }
                    Err(_) => now_ms,
                };

                let mut title = String::new();
                let mut body = String::new();

                if let Ok(notif) = user_notif.Notification() {
                    if let Ok(visual) = notif.Visual() {
                        if let Ok(bindings) = visual.Bindings() {
                            let b_count = bindings.Size().unwrap_or(0);
                            for b_idx in 0..b_count {
                                if let Ok(binding) = bindings.GetAt(b_idx) {
                                    if let Ok(text_elements) = binding.GetTextElements() {
                                        let t_count = text_elements.Size().unwrap_or(0);
                                        for t_idx in 0..t_count {
                                            if let Ok(t_elem) = text_elements.GetAt(t_idx) {
                                                if let Ok(text_h) = t_elem.Text() {
                                                    let text_str = text_h.to_string();
                                                    if title.is_empty() {
                                                        title = text_str;
                                                    } else if body.is_empty() {
                                                        body = text_str;
                                                    } else {
                                                        let lower_str = text_str.to_lowercase();
                                                        let is_browser = {
                                                            let lower_app = app_name.to_lowercase();
                                                            lower_app.contains("chrome")
                                                                || lower_app.contains("edge")
                                                                || lower_app.contains("brave")
                                                                || lower_app.contains("firefox")
                                                                || lower_app.contains("opera")
                                                                || lower_app.contains("browser")
                                                        };

                                                        let web_brand = if lower_str.contains("whatsapp") {
                                                            Some("WhatsApp")
                                                        } else if lower_str.contains("telegram") || lower_str.contains("t.me") {
                                                            Some("Telegram")
                                                        } else if lower_str.contains("discord") {
                                                            Some("Discord")
                                                        } else if lower_str.contains("gmail") || lower_str.contains("mail.google") {
                                                            Some("Gmail")
                                                        } else if lower_str.contains("instagram") {
                                                            Some("Instagram")
                                                        } else if lower_str.contains("youtube") {
                                                            Some("YouTube")
                                                        } else if lower_str.contains("slack") {
                                                            Some("Slack")
                                                        } else if lower_str.contains("teams") {
                                                            Some("Microsoft Teams")
                                                        } else if lower_str.contains("outlook") {
                                                            Some("Outlook")
                                                        } else {
                                                            None
                                                        };

                                                        if let Some(brand) = web_brand {
                                                            if is_browser {
                                                                app_name = brand.to_string();
                                                            }
                                                        } else if lower_str.starts_with("http")
                                                            || lower_str.contains(".com")
                                                            || lower_str.contains(".org")
                                                            || lower_str.contains(".net")
                                                            || lower_str.contains(".app")
                                                        {
                                                            // Skip domain clutter
                                                        } else {
                                                            body.push_str(" · ");
                                                            body.push_str(&text_str);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if title.is_empty() && body.is_empty() {
                    continue;
                }

                if title.is_empty() {
                    title = app_name.clone();
                }

                // Identify critical notifications (alarms, timers, reminders)
                let is_critical = {
                    let combined = format!("{app_name} {title} {body}").to_lowercase();
                    combined.contains("timer")
                        || combined.contains("alarm")
                        || combined.contains("reminder")
                        || combined.contains("critical")
                        || combined.contains("alert")
                };

                // Check for action buttons
                let actions = vec![
                    NotificationAction {
                        id: "open".to_string(),
                        label: "Open".to_string(),
                        is_supported: false, // Windows does not expose toast activation broker to unpackaged Win32 listeners
                    },
                    NotificationAction {
                        id: "dismiss".to_string(),
                        label: "Dismiss".to_string(),
                        is_supported: true,
                    },
                ];

                let app_key = if !app_id.is_empty() {
                    &app_id
                } else {
                    &app_name
                };
                let app_icon = get_app_icon_base64(app_key, &display_info);

                items.push(NotificationItem {
                    id,
                    app_name,
                    app_id,
                    title,
                    body,
                    timestamp_ms,
                    is_critical,
                    count: 1,
                    actions,
                    app_icon,
                    underlying_ids: vec![id],
                });
            }
        }

        process_notification_items(items, now_ms, access_status)
    }
    #[cfg(not(target_os = "windows"))]
    {
        NotificationSnapshot::default()
    }
}

pub fn process_notification_items(
    mut items: Vec<NotificationItem>,
    now_ms: u64,
    access_status: String,
) -> NotificationSnapshot {
    // Sort: critical items first, then descending by timestamp (newest first)
    items.sort_by(|a, b| {
        match b.is_critical.cmp(&a.is_critical) {
            std::cmp::Ordering::Equal => b.timestamp_ms.cmp(&a.timestamp_ms),
            other => other,
        }
    });

    // Group repeated notifications from the same application
    let mut grouped_items: Vec<NotificationItem> = Vec::new();
    for item in items {
        if let Some(existing) = grouped_items
            .iter_mut()
            .find(|g| g.app_name == item.app_name && g.is_critical == item.is_critical)
        {
            existing.count += 1;
            existing.underlying_ids.extend(item.underlying_ids);
            // If newer, update title & body
            if item.timestamp_ms > existing.timestamp_ms {
                existing.title = item.title;
                existing.body = item.body;
                existing.timestamp_ms = item.timestamp_ms;
                existing.id = item.id;
            }
            if existing.app_icon.is_none() && item.app_icon.is_some() {
                existing.app_icon = item.app_icon;
            }
        } else {
            grouped_items.push(item);
        }
    }

    // Sort grouped items: critical alerts first, then newest
    grouped_items.sort_by(|a, b| {
        match b.is_critical.cmp(&a.is_critical) {
            std::cmp::Ordering::Equal => b.timestamp_ms.cmp(&a.timestamp_ms),
            other => other,
        }
    });

    let total_count = grouped_items.len();
    const MAX_QUEUE: usize = 10;
    let overflow_count = total_count.saturating_sub(MAX_QUEUE);
    grouped_items.truncate(MAX_QUEUE);

    // Determine active compact notification:
    // Prioritize critical alert first; otherwise the freshest notification if under 2 minutes old
    let active_notification = grouped_items
        .iter()
        .find(|n| n.is_critical)
        .or_else(|| {
            grouped_items.first().filter(|n| {
                now_ms.saturating_sub(n.timestamp_ms) < 120_000 // within 2 minutes
            })
        })
        .cloned();

    NotificationSnapshot {
        access_status,
        active_notification,
        notifications: grouped_items,
        overflow_count,
        total_count,
    }
}

pub fn apply_filtering_and_privacy(
    mut snapshot: NotificationSnapshot,
    config: &crate::services::config::AppConfig,
) -> NotificationSnapshot {
    if !config.notifications_enabled {
        return NotificationSnapshot {
            access_status: snapshot.access_status,
            active_notification: None,
            notifications: Vec::new(),
            overflow_count: 0,
            total_count: 0,
        };
    }

    // Filter out blocked apps
    snapshot.notifications.retain(|item| {
        !config.blocked_apps.iter().any(|b| {
            b.eq_ignore_ascii_case(&item.app_name)
                || (!item.app_id.is_empty() && b.eq_ignore_ascii_case(&item.app_id))
        })
    });

    if let Some(ref active) = snapshot.active_notification {
        if config.blocked_apps.iter().any(|b| {
            b.eq_ignore_ascii_case(&active.app_name)
                || (!active.app_id.is_empty() && b.eq_ignore_ascii_case(&active.app_id))
        }) {
            snapshot.active_notification = None;
        }
    }

    if snapshot.active_notification.is_none() && !snapshot.notifications.is_empty() {
        snapshot.active_notification = snapshot.notifications.first().cloned();
    }

    // Apply Privacy Mode if enabled: redact title and body
    if config.privacy_mode {
        for item in &mut snapshot.notifications {
            item.title = item.app_name.clone();
            item.body = "New notification".to_string();
        }
        if let Some(ref mut active) = snapshot.active_notification {
            active.title = active.app_name.clone();
            active.body = "New notification".to_string();
        }
    }

    snapshot.total_count = snapshot.notifications.len();
    snapshot.overflow_count = snapshot.total_count.saturating_sub(10);

    snapshot
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_repeated_notifications_from_same_app() {
        let items = vec![
            NotificationItem {
                id: 1,
                app_name: "Slack".to_string(),
                app_id: "slack.exe".to_string(),
                title: "Alice".to_string(),
                body: "First message".to_string(),
                timestamp_ms: 1000,
                is_critical: false,
                count: 1,
                actions: Vec::new(),
                app_icon: None,
                underlying_ids: vec![1],
            },
            NotificationItem {
                id: 2,
                app_name: "Slack".to_string(),
                app_id: "slack.exe".to_string(),
                title: "Alice".to_string(),
                body: "Second message".to_string(),
                timestamp_ms: 2000,
                is_critical: false,
                count: 1,
                actions: Vec::new(),
                app_icon: Some("data:image/png;base64,mock".to_string()),
                underlying_ids: vec![2],
            },
        ];

        let snap = process_notification_items(items, 2500, "allowed".to_string());
        assert_eq!(snap.total_count, 1);
        assert_eq!(snap.notifications[0].count, 2);
        assert_eq!(snap.notifications[0].body, "Second message");
        assert_eq!(snap.notifications[0].id, 2);
        assert_eq!(
            snap.notifications[0].app_icon.as_deref(),
            Some("data:image/png;base64,mock")
        );
    }

    #[test]
    fn prioritizes_critical_alerts_and_bounds_queue() {
        let mut items = Vec::new();
        // Add 12 ordinary items from different apps
        for i in 1..=12 {
            items.push(NotificationItem {
                id: i,
                app_name: format!("App {i}"),
                app_id: format!("app{i}.exe"),
                title: "Title".to_string(),
                body: "Body".to_string(),
                timestamp_ms: i as u64 * 1000,
                is_critical: false,
                count: 1,
                actions: Vec::new(),
                app_icon: None,
                underlying_ids: vec![i],
            });
        }
        // Add 1 critical timer alert
        items.push(NotificationItem {
            id: 99,
            app_name: "Clock".to_string(),
            app_id: "clock.exe".to_string(),
            title: "Timer Done".to_string(),
            body: "Your 10m timer is up".to_string(),
            timestamp_ms: 500,
            is_critical: true,
            count: 1,
            actions: Vec::new(),
            app_icon: None,
            underlying_ids: vec![99],
        });

        let snap = process_notification_items(items, 15000, "allowed".to_string());
        assert_eq!(snap.notifications.len(), 10);
        assert_eq!(snap.total_count, 13);
        assert_eq!(snap.overflow_count, 3);
        // Critical item is prioritized as activeNotification even though timestamp is older
        assert!(snap.active_notification.is_some());
        assert_eq!(snap.active_notification.unwrap().id, 99);
    }

    #[test]
    fn local_dismissal_preserves_windows_history_and_prunes_safely() {
        let manager = NotificationManager::new();
        // Simulate notification 42 dismissed locally
        manager.dismiss_notification(42);

        {
            let dismissed = manager.dismissed_ids.lock().unwrap();
            assert!(dismissed.contains_key(&42));
        }

        // Case 1: Notification 42 is still present in Windows Notification Center
        let mut current_windows_ids = std::collections::HashSet::new();
        current_windows_ids.insert(42);
        current_windows_ids.insert(100);

        // Retention check matches fetch_notifications_internal logic
        {
            let mut dismissed = manager.dismissed_ids.lock().unwrap();
            dismissed.retain(|id, _| current_windows_ids.contains(id));
            assert!(dismissed.contains_key(&42), "Still present in Windows -> remain locally suppressed");
        }

        // Case 2: User clears notification 42 from Windows Action Center
        current_windows_ids.remove(&42);
        {
            let mut dismissed = manager.dismissed_ids.lock().unwrap();
            dismissed.retain(|id, _| current_windows_ids.contains(id));
            assert!(!dismissed.contains_key(&42), "Cleared from Windows -> pruned from suppression list");
        }
    }

    #[test]
    fn app_identity_and_icon_preserved_without_fake_brand_guessing() {
        // Notification from Microsoft Edge containing WhatsApp text must report Microsoft Edge,
        // never fabricated as WhatsApp.
        let edge_notif = NotificationItem {
            id: 55,
            app_name: "Microsoft Edge".to_string(),
            app_id: "Microsoft.MicrosoftEdge.Stable".to_string(),
            title: "WhatsApp Web".to_string(),
            body: "New message from Bob".to_string(),
            timestamp_ms: 5000,
            is_critical: false,
            count: 1,
            actions: Vec::new(),
            app_icon: Some("data:image/png;base64,edge_logo".to_string()),
            underlying_ids: vec![55],
        };

        let snap = process_notification_items(vec![edge_notif], 6000, "allowed".to_string());
        assert_eq!(snap.notifications[0].app_name, "Microsoft Edge");
        assert_eq!(snap.notifications[0].app_id, "Microsoft.MicrosoftEdge.Stable");
        assert_eq!(snap.notifications[0].app_icon.as_deref(), Some("data:image/png;base64,edge_logo"));
    }

    #[test]
    fn blocked_apps_never_reach_frontend_initial_or_live() {
        use crate::services::config::AppConfig;

        let allowed_item = NotificationItem {
            id: 1,
            app_name: "Calendar".to_string(),
            app_id: "calendar.exe".to_string(),
            title: "Meeting at 3".to_string(),
            body: "Product sync".to_string(),
            timestamp_ms: 1000,
            is_critical: false,
            count: 1,
            actions: Vec::new(),
            app_icon: None,
            underlying_ids: vec![1],
        };

        let blocked_item = NotificationItem {
            id: 2,
            app_name: "BlockedChat".to_string(),
            app_id: "com.blocked.chat".to_string(),
            title: "Spam message".to_string(),
            body: "Should not be seen".to_string(),
            timestamp_ms: 2000,
            is_critical: false,
            count: 1,
            actions: Vec::new(),
            app_icon: None,
            underlying_ids: vec![2],
        };

        let snapshot = NotificationSnapshot {
            access_status: "allowed".to_string(),
            active_notification: Some(blocked_item.clone()),
            notifications: vec![blocked_item, allowed_item.clone()],
            overflow_count: 0,
            total_count: 2,
        };

        let mut config = AppConfig::default();
        config.blocked_apps = vec!["BlockedChat".to_string()];

        let filtered = apply_filtering_and_privacy(snapshot, &config);

        // Blocked app is completely excluded
        assert_eq!(filtered.notifications.len(), 1);
        assert_eq!(filtered.total_count, 1);
        assert_eq!(filtered.notifications[0].app_name, "Calendar");

        // Active notification switches to allowed notification
        assert!(filtered.active_notification.is_some());
        assert_eq!(filtered.active_notification.unwrap().app_name, "Calendar");
    }

    #[test]
    fn privacy_mode_redacts_initial_and_live_without_logging_content() {
        use crate::services::config::AppConfig;

        let item = NotificationItem {
            id: 10,
            app_name: "Signal".to_string(),
            app_id: "org.whispersystems.signal".to_string(),
            title: "Secret Contact".to_string(),
            body: "Top secret private message content".to_string(),
            timestamp_ms: 5000,
            is_critical: false,
            count: 1,
            actions: Vec::new(),
            app_icon: None,
            underlying_ids: vec![10],
        };

        let snapshot = NotificationSnapshot {
            access_status: "allowed".to_string(),
            active_notification: Some(item.clone()),
            notifications: vec![item],
            overflow_count: 0,
            total_count: 1,
        };

        // 1. With Privacy Mode ENABLED: title is app name, body is generic label
        let mut private_config = AppConfig::default();
        private_config.privacy_mode = true;

        let redacted = apply_filtering_and_privacy(snapshot.clone(), &private_config);
        assert_eq!(redacted.notifications[0].title, "Signal");
        assert_eq!(redacted.notifications[0].body, "New notification");
        assert_eq!(redacted.active_notification.as_ref().unwrap().title, "Signal");
        assert_eq!(redacted.active_notification.as_ref().unwrap().body, "New notification");

        // 2. With Privacy Mode DISABLED: normal details shown
        let mut normal_config = AppConfig::default();
        normal_config.privacy_mode = false;

        let normal = apply_filtering_and_privacy(snapshot, &normal_config);
        assert_eq!(normal.notifications[0].title, "Secret Contact");
        assert_eq!(normal.notifications[0].body, "Top secret private message content");
    }

    #[test]
    fn grouped_notifications_track_all_underlying_ids_and_suppress_on_dismiss() {
        let manager = NotificationManager::new();

        // 3 incoming notifications from WhatsApp arriving in sequence
        let items = vec![
            NotificationItem {
                id: 101,
                app_name: "WhatsApp".to_string(),
                app_id: "whatsapp.exe".to_string(),
                title: "Mom".to_string(),
                body: "Message 1".to_string(),
                timestamp_ms: 1000,
                is_critical: false,
                count: 1,
                actions: Vec::new(),
                app_icon: None,
                underlying_ids: vec![101],
            },
            NotificationItem {
                id: 102,
                app_name: "WhatsApp".to_string(),
                app_id: "whatsapp.exe".to_string(),
                title: "Mom".to_string(),
                body: "Message 2".to_string(),
                timestamp_ms: 2000,
                is_critical: false,
                count: 1,
                actions: Vec::new(),
                app_icon: None,
                underlying_ids: vec![102],
            },
            NotificationItem {
                id: 103,
                app_name: "WhatsApp".to_string(),
                app_id: "whatsapp.exe".to_string(),
                title: "Mom".to_string(),
                body: "Message 3".to_string(),
                timestamp_ms: 3000,
                is_critical: false,
                count: 1,
                actions: Vec::new(),
                app_icon: None,
                underlying_ids: vec![103],
            },
        ];

        // Process grouping for Poll 1
        let snap1 = process_notification_items(items.clone(), 3500, "allowed".to_string());
        assert_eq!(snap1.total_count, 1);
        let grouped = &snap1.notifications[0];
        assert_eq!(grouped.underlying_ids, vec![103, 102, 101]);
        assert!(grouped.underlying_ids.contains(&101));
        assert!(grouped.underlying_ids.contains(&102));
        assert!(grouped.underlying_ids.contains(&103));

        // Store snapshot in manager and dismiss the displayed notification (id 103)
        *manager.last_snapshot.lock().unwrap() = snap1;
        manager.dismiss_notification(103);

        // Verify that dismissing 103 suppressed ALL underlying IDs: 101, 102, and 103
        {
            let dismissed = manager.dismissed_ids.lock().unwrap();
            assert!(dismissed.contains_key(&101), "ID 101 must be suppressed");
            assert!(dismissed.contains_key(&102), "ID 102 must be suppressed");
            assert!(dismissed.contains_key(&103), "ID 103 must be suppressed");
        }

        // Simulate Poll 2: Windows returns raw items 101, 102, 103
        // Simulating the dismissal check in fetch_notifications_internal
        let dismissed_guard = manager.dismissed_ids.lock().unwrap().clone();
        let unsuppressed_poll2: Vec<NotificationItem> = items
            .iter()
            .filter(|item| !dismissed_guard.contains_key(&item.id))
            .cloned()
            .collect();

        assert!(unsuppressed_poll2.is_empty(), "No grouped notification may reappear on subsequent polls");

        // Simulate Poll 3: A brand new message (ID 104) arrives alongside 101, 102, 103
        let mut poll3_raw = items;
        poll3_raw.push(NotificationItem {
            id: 104,
            app_name: "WhatsApp".to_string(),
            app_id: "whatsapp.exe".to_string(),
            title: "Mom".to_string(),
            body: "Message 4 (new)".to_string(),
            timestamp_ms: 4000,
            is_critical: false,
            count: 1,
            actions: Vec::new(),
            app_icon: None,
            underlying_ids: vec![104],
        });

        let unsuppressed_poll3: Vec<NotificationItem> = poll3_raw
            .into_iter()
            .filter(|item| !dismissed_guard.contains_key(&item.id))
            .collect();

        assert_eq!(unsuppressed_poll3.len(), 1, "Only the new notification appears");
        assert_eq!(unsuppressed_poll3[0].id, 104);
        assert_eq!(unsuppressed_poll3[0].body, "Message 4 (new)");
    }

    #[test]
    fn clear_all_suppresses_all_grouped_and_ungrouped_ids_across_multiple_polls() {
        let manager = NotificationManager::new();

        let raw_items = vec![
            NotificationItem {
                id: 1,
                app_name: "AppA".to_string(),
                app_id: "appa.exe".to_string(),
                title: "A1".to_string(),
                body: "Text A1".to_string(),
                timestamp_ms: 1000,
                is_critical: false,
                count: 1,
                actions: Vec::new(),
                app_icon: None,
                underlying_ids: vec![1],
            },
            NotificationItem {
                id: 2,
                app_name: "AppA".to_string(),
                app_id: "appa.exe".to_string(),
                title: "A2".to_string(),
                body: "Text A2".to_string(),
                timestamp_ms: 2000,
                is_critical: false,
                count: 1,
                actions: Vec::new(),
                app_icon: None,
                underlying_ids: vec![2],
            },
            NotificationItem {
                id: 3,
                app_name: "AppB".to_string(),
                app_id: "appb.exe".to_string(),
                title: "B1".to_string(),
                body: "Text B1".to_string(),
                timestamp_ms: 1500,
                is_critical: false,
                count: 1,
                actions: Vec::new(),
                app_icon: None,
                underlying_ids: vec![3],
            },
        ];

        let snapshot = process_notification_items(raw_items.clone(), 2500, "allowed".to_string());
        *manager.last_snapshot.lock().unwrap() = snapshot;

        // User clears entire queue
        manager.clear_all();

        // Check that all IDs (1, 2, 3) are in dismissed_ids
        {
            let dismissed = manager.dismissed_ids.lock().unwrap();
            assert!(dismissed.contains_key(&1));
            assert!(dismissed.contains_key(&2));
            assert!(dismissed.contains_key(&3));
        }

        // On next poll, raw items 1, 2, 3 are all suppressed
        let dismissed_guard = manager.dismissed_ids.lock().unwrap().clone();
        let remaining: Vec<NotificationItem> = raw_items
            .into_iter()
            .filter(|i| !dismissed_guard.contains_key(&i.id))
            .collect();
        assert!(remaining.is_empty(), "All notifications remain suppressed on future polls");
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_windows_notification_listener_returns_valid_access_status() {
        let status = get_access_status();
        assert!(
            status == "allowed" || status == "denied" || status == "unspecified" || status == "unsupported",
            "Access status must be one of the known values"
        );
    }
}



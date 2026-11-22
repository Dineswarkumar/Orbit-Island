//! Windows toast notification listener service (channel-based, queue cap 20).

use crate::media::ComGuard;
use crossbeam_channel::Sender;
use orbit_core::config::Config;
use orbit_core::events::{
    CoreEvent, NotificationAction, NotificationItem, NotificationSnapshot, WakeFn,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const MAX_NOTIFICATION_QUEUE: usize = 20;

pub struct NotificationService {
    stop: Arc<AtomicBool>,
    park: Arc<(Mutex<()>, Condvar)>,
    last_snapshot: Arc<Mutex<NotificationSnapshot>>,
    dismissed_ids: Arc<Mutex<HashMap<u32, u64>>>,
    thread: Option<JoinHandle<()>>,
}

impl NotificationService {
    pub fn spawn(
        tx: Sender<CoreEvent>,
        wake: WakeFn,
        config: Arc<RwLock<Config>>,
    ) -> std::io::Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let park = Arc::new((Mutex::new(()), Condvar::new()));
        let last_snapshot = Arc::new(Mutex::new(NotificationSnapshot::default()));
        let dismissed_ids = Arc::new(Mutex::new(HashMap::new()));

        let st = stop.clone();
        let pk = park.clone();
        let ls = last_snapshot.clone();
        let dm = dismissed_ids.clone();

        let thread = std::thread::Builder::new()
            .name("orbit-notifications".into())
            .spawn(move || {
                let _com = ComGuard::new();
                #[cfg(target_os = "windows")]
                let mut listener_cache: Option<
                    windows::UI::Notifications::Management::UserNotificationListener,
                > = None;
                #[cfg(not(target_os = "windows"))]
                let mut listener_cache: Option<()> = None;

                while !st.load(Ordering::Relaxed) {
                    let cfg = config.read().unwrap_or_else(|e| e.into_inner()).clone();
                    let raw_snapshot =
                        fetch_notifications_internal(&dm, &mut listener_cache, cfg.notifications_enabled);
                    let snapshot = apply_filtering_and_privacy(raw_snapshot, &cfg);

                    let changed = {
                        let mut guard = ls.lock().unwrap_or_else(|e| e.into_inner());
                        if *guard != snapshot {
                            *guard = snapshot.clone();
                            true
                        } else {
                            false
                        }
                    };

                    if changed {
                        if tx.send(CoreEvent::NotificationsUpdated(snapshot)).is_err() {
                            break;
                        }
                        wake();
                    }

                    let guard = pk.0.lock().unwrap_or_else(|e| e.into_inner());
                    if !st.load(Ordering::Relaxed) {
                        let _ = pk.1.wait_timeout(guard, Duration::from_millis(1000));
                    }
                }
            })?;

        Ok(Self {
            stop,
            park,
            last_snapshot,
            dismissed_ids,
            thread: Some(thread),
        })
    }

    pub fn current_snapshot(&self) -> NotificationSnapshot {
        self.last_snapshot
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn dismiss_notification(&self, id: u32) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let mut dismissed = self
            .dismissed_ids
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        dismissed.insert(id, now);

        let snapshot = self
            .last_snapshot
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        for item in &snapshot.notifications {
            if item.id == id || item.underlying_ids.contains(&id) {
                for &uid in &item.underlying_ids {
                    dismissed.insert(uid, now);
                }
            }
        }
        self.park.1.notify_all();
    }

    pub fn clear_all(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let current = self
            .last_snapshot
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let mut dismissed = self
            .dismissed_ids
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        for n in &current.notifications {
            dismissed.insert(n.id, now);
            for &uid in &n.underlying_ids {
                dismissed.insert(uid, now);
            }
        }
        self.park.1.notify_all();
    }
}

impl Drop for NotificationService {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.park.1.notify_all();
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

pub fn get_access_status() -> String {
    #[cfg(target_os = "windows")]
    {
        use windows::UI::Notifications::Management::{
            UserNotificationListener, UserNotificationListenerAccessStatus,
        };
        let _com = ComGuard::new();
        match UserNotificationListener::Current() {
            Ok(listener) => match listener.GetAccessStatus() {
                Ok(UserNotificationListenerAccessStatus::Allowed) => "allowed".to_string(),
                Ok(UserNotificationListenerAccessStatus::Denied) => "denied".to_string(),
                Ok(UserNotificationListenerAccessStatus::Unspecified) => "unspecified".to_string(),
                _ => "unspecified".to_string(),
            },
            Err(e) => {
                tracing::warn!("UserNotificationListener not available: {e}");
                "unsupported".to_string()
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        "unsupported".to_string()
    }
}

pub fn request_access_blocking() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        use windows::UI::Notifications::Management::{
            UserNotificationListener, UserNotificationListenerAccessStatus,
        };
        let _com = ComGuard::new()?;
        let listener = UserNotificationListener::Current()
            .map_err(|e| format!("UserNotificationListener::Current failed: {e}"))?;
        let status = listener
            .RequestAccessAsync()
            .map_err(|e| format!("Failed to initiate RequestAccessAsync: {e}"))?
            .join()
            .map_err(|e| format!("RequestAccessAsync failed: {e}"))?;
        Ok(match status {
            UserNotificationListenerAccessStatus::Allowed => "allowed",
            UserNotificationListenerAccessStatus::Denied => "denied",
            UserNotificationListenerAccessStatus::Unspecified => "unspecified",
            _ => "unspecified",
        }
        .to_string())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok("unsupported".to_string())
    }
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
    const MAX_ICON_CACHE: usize = 64;

    {
        let mut cache_guard = APP_ICON_CACHE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let cache = cache_guard.get_or_insert_with(HashMap::new);
        if let Some(cached) = cache.get(app_key) {
            return Some(cached.clone());
        }
    }

    let loaded = (|| {
        use base64::Engine;
        let d_info = display_info.as_ref()?;
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
            if let Ok(logo_ref) = d_info.GetLogo(size)
                && let Ok(stream_op) = logo_ref.OpenReadAsync()
                && let Ok(stream) = stream_op.join()
            {
                let stream_size = stream.Size().unwrap_or(0);
                if stream_size > 0 && stream_size <= 2 * 1024 * 1024 {
                    stream_opt = Some((stream, stream_size));
                    break;
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
        let mut cache_guard = APP_ICON_CACHE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let cache = cache_guard.get_or_insert_with(HashMap::new);
        if cache.len() >= MAX_ICON_CACHE {
            cache.clear();
        }
        cache.insert(app_key.to_string(), icon_data.clone());
    }
    loaded
}

fn is_priority_notification(app_name: &str, app_id: &str, title: &str, body: &str) -> bool {
    let source = format!("{app_name} {app_id}").to_lowercase();
    let message = format!("{title} {body}").to_lowercase();
    let is_time_or_reminder_app = source.contains("clock")
        || source.contains("windowsalarms")
        || source.contains("calendar")
        || source.contains("reminder");
    if !is_time_or_reminder_app {
        return false;
    }
    ["timer", "alarm", "reminder"]
        .iter()
        .any(|word| message.contains(word))
}

#[cfg(target_os = "windows")]
fn fetch_notifications_internal(
    dismissed_store: &Arc<Mutex<HashMap<u32, u64>>>,
    listener_cache: &mut Option<windows::UI::Notifications::Management::UserNotificationListener>,
    poll_notifications: bool,
) -> NotificationSnapshot {
    use windows::UI::Notifications::Management::{
        UserNotificationListener, UserNotificationListenerAccessStatus,
    };
    use windows::UI::Notifications::NotificationKinds;

    if listener_cache.is_none() {
        *listener_cache = UserNotificationListener::Current().ok();
    }
    let Some(listener) = listener_cache.as_ref() else {
        return NotificationSnapshot {
            access_status: "unsupported".to_string(),
            ..Default::default()
        };
    };

    let access_status = match listener.GetAccessStatus() {
        Ok(UserNotificationListenerAccessStatus::Allowed) => "allowed".to_string(),
        Ok(UserNotificationListenerAccessStatus::Denied) => {
            return NotificationSnapshot {
                access_status: "denied".to_string(),
                ..Default::default()
            };
        }
        Ok(_) => {
            return NotificationSnapshot {
                access_status: "unspecified".to_string(),
                ..Default::default()
            };
        }
        Err(_) => {
            *listener_cache = None;
            return NotificationSnapshot {
                access_status: "unspecified".to_string(),
                ..Default::default()
            };
        }
    };

    if !poll_notifications {
        return NotificationSnapshot {
            access_status,
            ..Default::default()
        };
    }

    let async_op = match listener.GetNotificationsAsync(NotificationKinds::Toast) {
        Ok(op) => op,
        Err(e) => {
            tracing::warn!("GetNotificationsAsync failed: {e}");
            *listener_cache = None;
            return NotificationSnapshot {
                access_status,
                ..Default::default()
            };
        }
    };

        let raw_notifications = match async_op.join() {
            Ok(list) => list,
            Err(e) => {
                tracing::warn!("Awaiting GetNotificationsAsync failed: {e}");
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
            if let Ok(user_notif) = raw_notifications.GetAt(i)
                && let Ok(id) = user_notif.Id()
            {
                current_windows_ids.insert(id);
            }
        }

        {
            let mut dismissed = dismissed_store
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            dismissed.retain(|id, _| current_windows_ids.contains(id));
        }

        let dismissed = dismissed_store
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
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

                if let Ok(notif) = user_notif.Notification()
                    && let Ok(visual) = notif.Visual()
                    && let Ok(bindings) = visual.Bindings()
                {
                    let b_count = bindings.Size().unwrap_or(0);
                    for b_idx in 0..b_count {
                        if let Ok(binding) = bindings.GetAt(b_idx)
                            && let Ok(text_elements) = binding.GetTextElements()
                        {
                            let t_count = text_elements.Size().unwrap_or(0);
                            for t_idx in 0..t_count {
                                if let Ok(t_elem) = text_elements.GetAt(t_idx)
                                    && let Ok(text_h) = t_elem.Text()
                                {
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
                                            Some("WhatsApp".to_string())
                                        } else if lower_str.contains("telegram")
                                            || lower_str.contains("t.me")
                                        {
                                            Some("Telegram".to_string())
                                        } else if lower_str.contains("discord") {
                                            Some("Discord".to_string())
                                        } else if lower_str.contains("gmail")
                                            || lower_str.contains("mail.google")
                                        {
                                            Some("Gmail".to_string())
                                        } else if lower_str.contains("calendar.google") {
                                            Some("Google Calendar".to_string())
                                        } else if lower_str.contains("drive.google")
                                            || lower_str.contains("docs.google")
                                        {
                                            Some("Google Drive".to_string())
                                        } else if lower_str.contains("meet.google") {
                                            Some("Google Meet".to_string())
                                        } else if lower_str.contains("instagram") {
                                            Some("Instagram".to_string())
                                        } else if lower_str.contains("music.youtube")
                                            || lower_str.contains("youtube music")
                                        {
                                            Some("YouTube Music".to_string())
                                        } else if lower_str.contains("youtube") {
                                            Some("YouTube".to_string())
                                        } else if lower_str.contains("github") {
                                            Some("GitHub".to_string())
                                        } else if lower_str.contains("linkedin") {
                                            Some("LinkedIn".to_string())
                                        } else if lower_str.contains("chatgpt")
                                            || lower_str.contains("openai")
                                        {
                                            Some("ChatGPT".to_string())
                                        } else if lower_str.contains("x.com")
                                            || lower_str.contains("twitter")
                                        {
                                            Some("X".to_string())
                                        } else if lower_str.contains("reddit") {
                                            Some("Reddit".to_string())
                                        } else if lower_str.contains("spotify") {
                                            Some("Spotify".to_string())
                                        } else if lower_str.contains("slack") {
                                            Some("Slack".to_string())
                                        } else if lower_str.contains("teams") {
                                            Some("Microsoft Teams".to_string())
                                        } else if lower_str.contains("outlook")
                                            || lower_str.contains("office.com")
                                            || lower_str.contains("live.com")
                                        {
                                            Some("Outlook".to_string())
                                        } else {
                                            None
                                        };

                                        if let Some(brand) = web_brand {
                                            if is_browser {
                                                app_name = brand;
                                            }
                                        } else if lower_str.starts_with("http")
                                            || lower_str.contains(".com")
                                            || lower_str.contains(".org")
                                            || lower_str.contains(".net")
                                            || lower_str.contains(".app")
                                            || lower_str.contains(".io")
                                            || lower_str.contains(".dev")
                                            || lower_str.contains(".in")
                                            || lower_str.contains(".edu")
                                        {
                                            if is_browser {
                                                let host = lower_str
                                                    .trim_start_matches("https://")
                                                    .trim_start_matches("http://")
                                                    .split('/')
                                                    .next()
                                                    .unwrap_or("")
                                                    .split(':')
                                                    .next()
                                                    .unwrap_or("");
                                                let parts: Vec<&str> = host
                                                    .split('.')
                                                    .filter(|p| {
                                                        !p.is_empty()
                                                            && *p != "www"
                                                            && *p != "web"
                                                            && *p != "app"
                                                            && *p != "m"
                                                    })
                                                    .collect();
                                                if let Some(&domain_core) = parts.first()
                                                    && domain_core.len() >= 3
                                                {
                                                    let mut chars = domain_core.chars();
                                                    if let Some(first) = chars.next() {
                                                        app_name = first
                                                            .to_uppercase()
                                                            .collect::<String>()
                                                            + chars.as_str();
                                                    }
                                                }
                                            }
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

                if title.is_empty() && body.is_empty() {
                    continue;
                }
                if title.is_empty() {
                    title = app_name.clone();
                }

                let is_critical = is_priority_notification(&app_name, &app_id, &title, &body);
                let actions = vec![
                    NotificationAction {
                        id: "open".to_string(),
                        label: "Open".to_string(),
                        is_supported: false,
                    },
                    NotificationAction {
                        id: "dismiss".to_string(),
                        label: "Dismiss".to_string(),
                        is_supported: true,
                    },
                ];

                let app_key = if !app_id.is_empty() { &app_id } else { &app_name };
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
fn fetch_notifications_internal(
    _dismissed_store: &Arc<Mutex<HashMap<u32, u64>>>,
    _listener_cache: &mut Option<()>,
    _poll_notifications: bool,
) -> NotificationSnapshot {
    NotificationSnapshot::default()
}

pub fn process_notification_items(
    mut items: Vec<NotificationItem>,
    now_ms: u64,
    access_status: String,
) -> NotificationSnapshot {
    items.sort_by(|a, b| match b.is_critical.cmp(&a.is_critical) {
        std::cmp::Ordering::Equal => b.timestamp_ms.cmp(&a.timestamp_ms),
        other => other,
    });

    let mut grouped_items: Vec<NotificationItem> = Vec::new();
    for item in items {
        if let Some(existing) = grouped_items.iter_mut().find(|g| {
            let same_display_name = g.app_name.eq_ignore_ascii_case(&item.app_name);
            let same_app_id = g.app_id.is_empty()
                || item.app_id.is_empty()
                || g.app_id.eq_ignore_ascii_case(&item.app_id);
            same_display_name && same_app_id && g.is_critical == item.is_critical
        }) {
            existing.count += 1;
            existing.underlying_ids.extend(item.underlying_ids);
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

    grouped_items.sort_by(|a, b| match b.is_critical.cmp(&a.is_critical) {
        std::cmp::Ordering::Equal => b.timestamp_ms.cmp(&a.timestamp_ms),
        other => other,
    });

    let total_count = grouped_items.len();
    let overflow_count = total_count.saturating_sub(MAX_NOTIFICATION_QUEUE);
    grouped_items.truncate(MAX_NOTIFICATION_QUEUE);

    let active_notification = grouped_items
        .iter()
        .find(|n| n.is_critical)
        .or_else(|| {
            grouped_items
                .first()
                .filter(|n| now_ms.saturating_sub(n.timestamp_ms) < 120_000)
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
    config: &Config,
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

    snapshot.notifications.retain(|item| {
        !config.blocked_apps.iter().any(|b| {
            b.eq_ignore_ascii_case(&item.app_name)
                || (!item.app_id.is_empty() && b.eq_ignore_ascii_case(&item.app_id))
        })
    });

    if let Some(ref active) = snapshot.active_notification
        && config.blocked_apps.iter().any(|b| {
            b.eq_ignore_ascii_case(&active.app_name)
                || (!active.app_id.is_empty() && b.eq_ignore_ascii_case(&active.app_id))
        })
    {
        snapshot.active_notification = None;
    }

    if snapshot.active_notification.is_none() && !snapshot.notifications.is_empty() {
        snapshot.active_notification = snapshot.notifications.first().cloned();
    }

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
    snapshot.overflow_count = snapshot.total_count.saturating_sub(MAX_NOTIFICATION_QUEUE);
    snapshot
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_item(id: u32, ts: u64) -> NotificationItem {
        NotificationItem {
            id,
            app_name: format!("App {id}"),
            app_id: format!("app{id}.exe"),
            title: "Title".to_string(),
            body: "Body".to_string(),
            timestamp_ms: ts,
            is_critical: false,
            count: 1,
            actions: Vec::new(),
            app_icon: None,
            underlying_ids: vec![id],
        }
    }

    #[test]
    fn queue_cap_20_drops_oldest_when_full() {
        let mut items = Vec::new();
        for i in 1..=25u32 {
            items.push(mk_item(i, i as u64 * 1000));
        }
        let snap = process_notification_items(items, 30_000, "allowed".to_string());
        assert_eq!(snap.notifications.len(), 20);
        assert_eq!(snap.total_count, 25);
        assert_eq!(snap.overflow_count, 5);
        assert_eq!(snap.notifications[0].id, 25);
        assert_eq!(snap.notifications[19].id, 6);
    }

    #[test]
    fn blocked_apps_and_privacy_mode_apply_cleanly() {
        let allowed = mk_item(1, 1000);
        let mut blocked = mk_item(2, 2000);
        blocked.app_name = "BlockedChat".to_string();

        let snap = NotificationSnapshot {
            access_status: "allowed".to_string(),
            active_notification: Some(blocked.clone()),
            notifications: vec![blocked, allowed],
            overflow_count: 0,
            total_count: 2,
        };

        let mut cfg = Config::default();
        cfg.blocked_apps = vec!["BlockedChat".to_string()];
        cfg.privacy_mode = true;

        let out = apply_filtering_and_privacy(snap, &cfg);
        assert_eq!(out.notifications.len(), 1);
        assert_eq!(out.notifications[0].title, "App 1");
        assert_eq!(out.notifications[0].body, "New notification");
    }
}

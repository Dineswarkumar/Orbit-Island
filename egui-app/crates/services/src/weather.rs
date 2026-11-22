//! Weather fetch service using `ureq` (no `curl.exe` child processes).

use crossbeam_channel::Sender;
use orbit_core::events::{CoreEvent, WakeFn, WeatherGlance};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

const CACHE_TTL: Duration = Duration::from_secs(600);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(6);
const WTTR_URL: &str = "https://wttr.in/?format=j1";

struct CachedWeather {
    data: WeatherGlance,
    fetched_at: Instant,
}

pub fn parse_wttr_json(json_str: &str) -> Result<WeatherGlance, String> {
    let val: serde_json::Value =
        serde_json::from_str(json_str).map_err(|e| format!("Invalid weather JSON: {e}"))?;

    let current = val
        .get("current_condition")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .ok_or_else(|| "Missing current_condition in weather data".to_string())?;

    let temp_c = current
        .get("temp_C")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<i32>().ok())
        .ok_or_else(|| "Missing or invalid temp_C".to_string())?;

    let temp_f = current
        .get("temp_F")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or_else(|| (temp_c * 9 / 5) + 32);

    let code = current
        .get("weatherCode")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(113);

    let condition_code = match code {
        200 | 386 | 389 | 392 | 395 => "storm",
        176 | 293 | 296 | 299 | 302 | 305 | 308 | 311 | 353 | 356 | 359 => "rain",
        179 | 182 | 227 | 230 | 323 | 326 | 329 | 332 | 335 | 338 | 368 | 371 => "snow",
        116 => "partly_cloudy",
        119 | 122 | 143 | 248 => "cloudy",
        _ => "clear",
    }
    .to_string();

    let label = current
        .get("weatherDesc")
        .and_then(|d| d.as_array())
        .and_then(|a| a.first())
        .and_then(|item| item.get("value"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Clear".to_string());

    Ok(WeatherGlance {
        temp_c,
        temp_f,
        condition_code,
        label,
    })
}

pub fn fetch_weather_http() -> Result<WeatherGlance, String> {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(REQUEST_TIMEOUT))
        .build();
    let agent: ureq::Agent = config.into();
    let body = agent
        .get(WTTR_URL)
        .header("User-Agent", "ORBIT-Desktop/1.0")
        .call()
        .map_err(|e| format!("Weather HTTP request failed: {e}"))?
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("Failed reading weather response: {e}"))?;
    parse_wttr_json(&body)
}

pub struct WeatherService {
    enabled: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    park: Arc<(Mutex<()>, Condvar)>,
    cache: Arc<Mutex<Option<CachedWeather>>>,
    thread: Option<JoinHandle<()>>,
}

impl WeatherService {
    pub fn spawn(
        tx: Sender<CoreEvent>,
        wake: WakeFn,
        enabled: Arc<AtomicBool>,
    ) -> std::io::Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let park = Arc::new((Mutex::new(()), Condvar::new()));
        let cache = Arc::new(Mutex::new(None::<CachedWeather>));

        let en = enabled.clone();
        let st = stop.clone();
        let pk = park.clone();
        let ch = cache.clone();

        let thread = std::thread::Builder::new()
            .name("orbit-weather".into())
            .spawn(move || {
                let mut last_sent: Option<WeatherGlance> = None;
                while !st.load(Ordering::Relaxed) {
                    if en.load(Ordering::Acquire) {
                        match fetch_weather_http() {
                            Ok(glance) => {
                                {
                                    let mut guard = ch.lock().unwrap_or_else(|e| e.into_inner());
                                    *guard = Some(CachedWeather {
                                        data: glance.clone(),
                                        fetched_at: Instant::now(),
                                    });
                                }
                                if last_sent.as_ref() != Some(&glance) {
                                    last_sent = Some(glance.clone());
                                    if tx.send(CoreEvent::WeatherUpdated(glance)).is_err() {
                                        break;
                                    }
                                    wake();
                                }
                            }
                            Err(e) => {
                                tracing::warn!("weather fetch failed: {e}");
                            }
                        }
                    }

                    let guard = pk.0.lock().unwrap_or_else(|e| e.into_inner());
                    if !st.load(Ordering::Relaxed) {
                        let _ = pk.1.wait_timeout(guard, CACHE_TTL);
                    }
                }
            })?;

        Ok(Self {
            enabled,
            stop,
            park,
            cache,
            thread: Some(thread),
        })
    }

    pub fn set_enabled(&self, enabled: bool) {
        let prev = self.enabled.swap(enabled, Ordering::AcqRel);
        if !prev && enabled {
            self.park.1.notify_all();
        }
    }

    pub fn refresh_now(&self) {
        self.park.1.notify_all();
    }

    pub fn get_weather(&self) -> Result<WeatherGlance, String> {
        {
            let guard = self.cache.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(ref c) = *guard
                && c.fetched_at.elapsed() < CACHE_TTL
            {
                return Ok(c.data.clone());
            }
        }
        let fresh = fetch_weather_http()?;
        {
            let mut guard = self.cache.lock().unwrap_or_else(|e| e.into_inner());
            *guard = Some(CachedWeather {
                data: fresh.clone(),
                fetched_at: Instant::now(),
            });
        }
        Ok(fresh)
    }
}

impl Drop for WeatherService {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.park.1.notify_all();
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_wttr_json_valid_and_rain() {
        let sample = r#"{
            "current_condition": [
                {
                    "temp_C": "21",
                    "temp_F": "70",
                    "weatherCode": "116",
                    "weatherDesc": [{ "value": "Partly cloudy" }]
                }
            ]
        }"#;
        let g = parse_wttr_json(sample).expect("valid wttr json");
        assert_eq!(g.temp_c, 21);
        assert_eq!(g.temp_f, 70);
        assert_eq!(g.condition_code, "partly_cloudy");
        assert_eq!(g.label, "Partly cloudy");

        let rain = r#"{
            "current_condition": [
                {
                    "temp_C": "18",
                    "temp_F": "64",
                    "weatherCode": "302",
                    "weatherDesc": [{ "value": "Moderate rain" }]
                }
            ]
        }"#;
        let g2 = parse_wttr_json(rain).expect("rain wttr json");
        assert_eq!(g2.condition_code, "rain");
    }

    #[test]
    fn parse_wttr_json_rejects_missing_fields() {
        assert!(parse_wttr_json("{}").is_err());
        assert!(parse_wttr_json("not json").is_err());
    }
}

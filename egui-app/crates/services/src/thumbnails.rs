//! Fixed-capacity LRU thumbnail cache for image files on the Shelf.
//! Strictly bounded to `MAX_THUMBNAIL_ENTRIES` items and images only (<= 64x64 RGBA).

use image::imageops::FilterType;
use std::collections::{HashMap, VecDeque};
use std::path::Path;

pub const MAX_THUMBNAIL_ENTRIES: usize = 24;
pub const THUMBNAIL_SIZE_PX: u32 = 64;

#[derive(Clone, Debug)]
pub struct ThumbnailRgba {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}

impl ThumbnailRgba {
    pub fn byte_len(&self) -> usize {
        self.rgba.len()
    }
}

pub fn is_supported_image_ext(ext: &str) -> bool {
    matches!(
        ext.trim().to_ascii_lowercase().as_str(),
        "png" | "jpg" | "jpeg"
    )
}

/// Decodes and center-crops/resizes an image file to at most `64x64` RGBA.
pub fn load_thumbnail_rgba(path: &Path) -> Option<ThumbnailRgba> {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    if !is_supported_image_ext(ext) {
        return None;
    }
    let meta = std::fs::metadata(path).ok()?;
    // Guard against huge raster bombs on shelf thumbnails (25 MB cap).
    if !meta.is_file() || meta.len() > 25 * 1024 * 1024 {
        return None;
    }
    let img = image::open(path).ok()?;
    let resized = img.resize_to_fill(THUMBNAIL_SIZE_PX, THUMBNAIL_SIZE_PX, FilterType::Triangle);
    let rgba = resized.to_rgba8();
    Some(ThumbnailRgba {
        width: rgba.width() as usize,
        height: rgba.height() as usize,
        rgba: rgba.into_raw(),
    })
}

/// Decodes encoded image bytes (PNG/JPEG) and resizes them to a square `size_px` RGBA buffer.
pub fn decode_image_bytes_rgba(bytes: &[u8], size_px: u32) -> Option<ThumbnailRgba> {
    let img = image::load_from_memory(bytes).ok()?;
    let sz = size_px.clamp(16, 256);
    let resized = img.resize_to_fill(sz, sz, FilterType::Lanczos3);
    let rgba = resized.to_rgba8();
    Some(ThumbnailRgba {
        width: rgba.width() as usize,
        height: rgba.height() as usize,
        rgba: rgba.into_raw(),
    })
}

/// Decodes a `data:image/...;base64,...` URI (or raw base64) and resizes it to `max_size_px` RGBA.
pub fn decode_data_uri_rgba(data_uri: &str, max_size_px: u32) -> Option<ThumbnailRgba> {
    use base64::Engine;
    let payload = data_uri
        .split_once(";base64,")
        .map_or(data_uri, |(_, b)| b)
        .trim();
    if payload.is_empty() || payload.len() > 8 * 1024 * 1024 {
        return None;
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(payload)
        .ok()?;
    decode_image_bytes_rgba(&bytes, max_size_px)
}

pub struct ThumbnailCache {
    capacity: usize,
    order: VecDeque<String>,
    entries: HashMap<String, ThumbnailRgba>,
}

impl Default for ThumbnailCache {
    fn default() -> Self {
        Self::new(MAX_THUMBNAIL_ENTRIES)
    }
}

impl ThumbnailCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            order: VecDeque::with_capacity(capacity.max(1)),
            entries: HashMap::with_capacity(capacity.max(1)),
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn total_bytes(&self) -> usize {
        self.entries.values().map(ThumbnailRgba::byte_len).sum()
    }

    pub fn contains(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    pub fn get(&mut self, key: &str) -> Option<&ThumbnailRgba> {
        if self.entries.contains_key(key) {
            self.touch(key);
            self.entries.get(key)
        } else {
            None
        }
    }

    /// Inserts a thumbnail into the cache and returns any keys evicted by LRU policy.
    pub fn insert(&mut self, key: String, thumb: ThumbnailRgba) -> Vec<String> {
        self.entries.insert(key.clone(), thumb);
        self.touch(&key);
        let mut evicted = Vec::new();
        while self.entries.len() > self.capacity {
            if let Some(old) = self.order.pop_front()
                && self.entries.remove(&old).is_some()
            {
                evicted.push(old);
            }
        }
        evicted
    }

    pub fn retain_keys(&mut self, keep: &std::collections::HashSet<String>) -> Vec<String> {
        let removed: Vec<String> = self
            .entries
            .keys()
            .filter(|k| !keep.contains(*k))
            .cloned()
            .collect();
        for k in &removed {
            self.entries.remove(k);
        }
        self.order.retain(|k| keep.contains(k));
        removed
    }

    pub fn clear(&mut self) {
        self.order.clear();
        self.entries.clear();
    }

    pub fn keys(&self) -> Vec<String> {
        self.order.iter().cloned().collect()
    }

    fn touch(&mut self, key: &str) {
        self.order.retain(|k| k != key);
        self.order.push_back(key.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lru_cache_stays_within_fixed_size_after_100_images() {
        let mut cache = ThumbnailCache::default();
        assert_eq!(cache.capacity(), MAX_THUMBNAIL_ENTRIES);

        let sample = ThumbnailRgba {
            width: THUMBNAIL_SIZE_PX as usize,
            height: THUMBNAIL_SIZE_PX as usize,
            rgba: vec![255u8; (THUMBNAIL_SIZE_PX * THUMBNAIL_SIZE_PX * 4) as usize],
        };

        for i in 0..100 {
            let _ = cache.insert(format!("C:/img/pic_{i}.png"), sample.clone());
        }

        assert_eq!(cache.len(), MAX_THUMBNAIL_ENTRIES);
        assert!(!cache.contains("C:/img/pic_0.png"));
        assert!(!cache.contains("C:/img/pic_75.png"));
        assert!(cache.contains("C:/img/pic_76.png"));
        assert!(cache.contains("C:/img/pic_99.png"));
        assert_eq!(
            cache.total_bytes(),
            MAX_THUMBNAIL_ENTRIES * (THUMBNAIL_SIZE_PX * THUMBNAIL_SIZE_PX * 4) as usize
        );
    }

    #[test]
    fn image_decode_loads_valid_png_and_skips_non_image() {
        let dir = std::env::temp_dir().join("orbit_thumb_test");
        let _ = std::fs::create_dir_all(&dir);

        let txt = dir.join("note.txt");
        std::fs::write(&txt, b"hello").unwrap();
        assert!(load_thumbnail_rgba(&txt).is_none());

        let png_path = dir.join("sample.png");
        let buf = image::RgbaImage::from_pixel(128, 96, image::Rgba([20, 120, 240, 255]));
        buf.save(&png_path).unwrap();
        let loaded = load_thumbnail_rgba(&png_path).expect("valid png should load");
        assert_eq!(loaded.width, THUMBNAIL_SIZE_PX as usize);
        assert_eq!(loaded.height, THUMBNAIL_SIZE_PX as usize);
        assert_eq!(loaded.byte_len(), (THUMBNAIL_SIZE_PX * THUMBNAIL_SIZE_PX * 4) as usize);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn data_uri_decode_handles_media_artwork_and_rejects_garbage() {
        use base64::Engine;
        let buf = image::RgbaImage::from_pixel(32, 32, image::Rgba([240, 80, 60, 255]));
        let mut png_bytes = Vec::new();
        image::DynamicImage::ImageRgba8(buf)
            .write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageFormat::Png)
            .unwrap();
        let uri = format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(&png_bytes)
        );
        let decoded = decode_data_uri_rgba(&uri, 48).expect("valid png data uri must decode");
        assert_eq!(decoded.width, 48);
        assert_eq!(decoded.height, 48);
        assert!(decode_data_uri_rgba("data:image/png;base64,not_valid_base64!!!", 48).is_none());
    }
}

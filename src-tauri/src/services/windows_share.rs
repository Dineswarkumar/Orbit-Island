//! Shares local files through the Windows Share UI so Windows can route them to
//! installed mail, messaging, nearby sharing, and notes applications.

#[cfg(target_os = "windows")]
use std::sync::{
    atomic::{AtomicI64, Ordering},
    Arc, Mutex, OnceLock,
};
#[cfg(target_os = "windows")]
use windows::ApplicationModel::DataTransfer::{DataRequestedEventArgs, DataTransferManager};
#[cfg(target_os = "windows")]
use windows::Foundation::TypedEventHandler;
#[cfg(target_os = "windows")]
use windows::Storage::{IStorageItem, StorageFile};
#[cfg(target_os = "windows")]
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED, COINIT_MULTITHREADED};
#[cfg(target_os = "windows")]
use windows::Win32::UI::Shell::IDataTransferManagerInterop;
#[cfg(target_os = "windows")]
use windows::core::{HSTRING, Interface, factory};
#[cfg(target_os = "windows")]
use windows_collections::IIterable;

#[cfg(target_os = "windows")]
static SHARE_HANDLERS: OnceLock<Mutex<std::collections::HashMap<isize, i64>>> = OnceLock::new();

#[cfg(target_os = "windows")]
pub struct SendStorageFiles(pub Vec<StorageFile>);
#[cfg(target_os = "windows")]
unsafe impl Send for SendStorageFiles {}
#[cfg(target_os = "windows")]
unsafe impl Sync for SendStorageFiles {}

#[cfg(target_os = "windows")]
impl SendStorageFiles {
    pub fn take(self) -> Vec<StorageFile> {
        self.0
    }
}

#[cfg(target_os = "windows")]
pub fn prepare_share_files(paths: &[String]) -> Result<Vec<StorageFile>, String> {
    if paths.is_empty() {
        return Err("Choose at least one file to share.".into());
    }

    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };

    paths
        .iter()
        .map(|path| {
            let h_path = HSTRING::from(path);
            StorageFile::GetFileFromPathAsync(&h_path)
                .and_then(|op| op.join())
                .map_err(|error| format!("Could not access file '{path}': {error}"))
        })
        .collect()
}

#[cfg(target_os = "windows")]
pub fn show_share_ui_for_window_on_main_thread(
    hwnd: isize,
    files: Vec<StorageFile>,
) -> Result<(), String> {
    let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };

    let interop = factory::<DataTransferManager, IDataTransferManagerInterop>()
        .map_err(|error| format!("Windows Share is unavailable: {error}"))?;
    let hwnd_val = windows::Win32::Foundation::HWND(hwnd as *mut core::ffi::c_void);
    let window_key = hwnd_val.0 as isize;
    let manager: DataTransferManager = unsafe { interop.GetForWindow(hwnd_val) }
        .map_err(|error| format!("Could not connect to Windows Share: {error}"))?;

    // Remove this one-shot handler after Windows requests the payload. This avoids
    // accumulating handlers when users share repeatedly or rapidly double-click.
    let token_slot = Arc::new(AtomicI64::new(0));
    let event_token = token_slot.clone();
    let files_holder = Arc::new(SendStorageFiles(files));
    let handler = TypedEventHandler::<DataTransferManager, DataRequestedEventArgs>::new(
        move |sender, args| {
            let files = &files_holder.0;
            let token = event_token.load(Ordering::Acquire);
            if token != 0 {
                if let Some(sender) = sender.as_ref() {
                    let _ = sender.RemoveDataRequested(token);
                }
                let handlers = SHARE_HANDLERS.get_or_init(|| Mutex::new(std::collections::HashMap::new()));
                let mut handlers = handlers.lock().unwrap_or_else(|poison| poison.into_inner());
                if handlers.get(&window_key) == Some(&token) {
                    handlers.remove(&window_key);
                }
            }

            let request = args.ok()?.Request()?;
            let deferral = request.GetDeferral()?;
            let data = request.Data()?;
            let properties = data.Properties()?;
            properties.SetTitle(&HSTRING::from("Share files from ORBIT"))?;

            let storage_items: Vec<Option<IStorageItem>> = files
                .iter()
                .map(|file| file.cast().map(Some))
                .collect::<windows::core::Result<_>>()?;
            let storage_items: IIterable<IStorageItem> = storage_items.into();
            data.SetStorageItems(&storage_items, true)?;
            deferral.Complete()?;
            Ok(())
        },
    );

    let handlers = SHARE_HANDLERS.get_or_init(|| Mutex::new(std::collections::HashMap::new()));
    let mut handlers = handlers.lock().unwrap_or_else(|poison| poison.into_inner());
    if let Some(previous_token) = handlers.remove(&window_key) {
        let _ = manager.RemoveDataRequested(previous_token);
    }
    let token = manager
        .DataRequested(&handler)
        .map_err(|error| format!("Could not prepare the Windows Share panel: {error}"))?;
    token_slot.store(token, Ordering::Release);
    handlers.insert(window_key, token);
    drop(handlers);

    if let Err(error) = unsafe { interop.ShowShareUIForWindow(hwnd_val) } {
        let _ = manager.RemoveDataRequested(token);
        let handlers = SHARE_HANDLERS.get_or_init(|| Mutex::new(std::collections::HashMap::new()));
        let mut handlers = handlers.lock().unwrap_or_else(|poison| poison.into_inner());
        if handlers.get(&window_key) == Some(&token) {
            handlers.remove(&window_key);
        }
        return Err(format!("Could not open the Windows Share panel: {error}"));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn share_files_for_window(hwnd: isize, paths: Vec<String>) -> Result<(), String> {
    let files = prepare_share_files(&paths)?;
    show_share_ui_for_window_on_main_thread(hwnd, files)
}

#[cfg(not(target_os = "windows"))]
pub fn share_files_for_window(_hwnd: isize, _paths: Vec<String>) -> Result<(), String> {
    Err("Windows app sharing is only available on Windows.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "windows")]
    fn test_prepare_share_files_empty() {
        let result = prepare_share_files(&[]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Choose at least one file to share.");
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_prepare_and_set_storage_items_across_apartments() {
        use windows::ApplicationModel::DataTransfer::DataPackage;
        let test_file = std::env::temp_dir().join(format!(
            "orbit-share-test-{}-{}.txt",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after UNIX epoch")
                .as_nanos()
        ));
        std::fs::write(&test_file, b"ORBIT share test")
            .expect("test fixture file should be writable");
        let path_str = test_file.to_string_lossy().into_owned();

        let files = prepare_share_files(&[path_str]).expect("prepare_share_files should succeed");
        let send_files = SendStorageFiles(files);

        let sta_result = std::thread::spawn(move || -> Result<(), String> {
            let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
            let files = send_files.take();
            let data = DataPackage::new().map_err(|e| format!("DataPackage::new failed: {e}"))?;
            let storage_items: Vec<Option<IStorageItem>> = files
                .iter()
                .map(|file| file.cast().map(Some))
                .collect::<windows::core::Result<_>>()
                .map_err(|e| format!("cast failed: {e}"))?;
            let storage_items: IIterable<IStorageItem> = storage_items.into();
            data.SetStorageItems(&storage_items, true)
                .map_err(|e| format!("SetStorageItems failed: {e}"))?;
            Ok(())
        })
        .join()
        .expect("STA thread panicked");
        let _ = std::fs::remove_file(&test_file);

        assert!(sta_result.is_ok(), "STA SetStorageItems failed: {:?}", sta_result);
    }
}

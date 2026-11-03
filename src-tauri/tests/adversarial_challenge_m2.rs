use orbit_lib::commands::verify_settings_window;
use orbit_lib::services::db::{validate_local_safe_path, DatabaseManager, DANGEROUS_EXTENSIONS};
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

#[test]
fn test_adversarial_unc_paths() {
    let unc_payloads = [
        r"\\127.0.0.1\c$\payload",
        "//server/share",
        r"\\localhost\c$",
        r"\\192.168.1.1\share\exploit.dll",
        r"\\?\UNC\server\share\file.txt",
        r"\\.\pipe\protected_pipe",
        "//127.0.0.1/admin$/system32",
        "//?/UNC/10.0.0.1/c$/boot.ini",
        r"\\wsl$\Ubuntu\etc\shadow",
        r"\\?\UNC\127.0.0.1\c$\test",
    ];

    for payload in unc_payloads {
        let res = validate_local_safe_path(payload);
        assert!(
            res.is_err(),
            "UNC attack payload was NOT rejected: '{}', got {:?}",
            payload,
            res
        );
        let err_msg = res.unwrap_err();
        println!("Rejected UNC '{payload}': {err_msg}");
    }
}

#[test]
fn test_adversarial_device_namespaces() {
    let device_payloads = [
        r"\\.\COM1",
        r"\\.\PhysicalDrive0",
        r"\\?\Volume{b75e2c83-0000-0000-0000-602200000000}\file",
        r"\\.\Volume{12345678-1234-1234-1234-123456789abc}",
        r"\\.\NUL",
        r"\\.\CON",
        r"\\.\C:\windows\system32",
        "//./COM1",
        "//./PhysicalDrive0",
        "//?/Volume{abc}",
        "//./pipe/docker_engine",
        r"\\?\C:\test.txt",
        r"\\.\GLOBALROOT\Device\HarddiskVolume1\payload.exe",
    ];

    for payload in device_payloads {
        let res = validate_local_safe_path(payload);
        assert!(
            res.is_err(),
            "Device namespace payload was NOT rejected: '{}', got {:?}",
            payload,
            res
        );
        let err_msg = res.unwrap_err();
        println!("Rejected Device Namespace '{payload}': {err_msg}");
    }
}

#[test]
fn test_adversarial_directory_traversal() {
    let traversal_payloads = [
        r"..\..\windows\system32\cmd.exe",
        "../relative/traversal",
        r"C:\safe\..\..\windows\system32\cmd.exe",
        r"C:\Windows\System32\..\..\..\boot.ini",
        r"folder\..\..\..\..\etc\passwd",
        r".\..\sensitive\secrets.json",
        r"C:/safe/../../windows/system32/cmd.exe",
        "../../../../../../../../../Windows/System32/calc.exe",
        r"subdir/../../target/debug",
        r"foo/bar/../../baz",
    ];

    for payload in traversal_payloads {
        let res = validate_local_safe_path(payload);
        assert!(
            res.is_err(),
            "Directory traversal payload was NOT rejected: '{}', got {:?}",
            payload,
            res
        );
        let err_msg = res.unwrap_err();
        println!("Rejected Traversal '{payload}': {err_msg}");
    }
}

#[test]
fn test_adversarial_dos_device_names() {
    let dos_devices = [
        "con", "CON", "cOn", "CoN",
        "prn", "PRN", "PrN",
        "aux", "AUX", "AuX",
        "nul", "NUL", "NuL",
        "com1", "COM1", "cOm1",
        "com2", "COM2",
        "com3", "COM3",
        "com4", "COM4",
        "com5", "COM5",
        "com6", "COM6",
        "com7", "COM7",
        "com8", "COM8",
        "com9", "COM9",
        "lpt1", "LPT1",
        "lpt2", "LPT2",
        "lpt3", "LPT3",
        "lpt4", "LPT4",
        "lpt5", "LPT5",
        "lpt6", "LPT6",
        "lpt7", "LPT7",
        "lpt8", "LPT8",
        "lpt9", "LPT9",
        // With extensions
        "con.txt", "CON.TXT", "con.dat",
        "prn.log", "PRN.EXE",
        "aux.cfg", "AUX.json",
        "nul.iso", "NUL.zip",
        "com1.bin", "COM9.dat",
        "lpt1.txt", "LPT9.out",
        // In nested paths
        r"C:\safe\folder\con",
        r"C:\safe\folder\con.txt",
        r"data\aux.dat",
        r"logs\prn\output.log",
        r"sub\dir\NUL",
    ];

    for payload in dos_devices {
        let res = validate_local_safe_path(payload);
        assert!(
            res.is_err(),
            "DOS device payload was NOT rejected: '{}', got {:?}",
            payload,
            res
        );
        let err_msg = res.unwrap_err();
        println!("Rejected DOS device '{payload}': {err_msg}");
    }
}

#[test]
fn test_adversarial_empty_and_null_bytes() {
    let empty_payloads = [
        "",
        "   ",
        "\t\t",
        "\r\n",
        "test\0payload",
        "\0malicious",
        "C:\\Windows\\\0System32",
    ];

    for payload in empty_payloads {
        let res = validate_local_safe_path(payload);
        assert!(
            res.is_err(),
            "Empty/null byte payload was NOT rejected: '{:?}', got {:?}",
            payload,
            res
        );
    }
}

#[test]
fn test_valid_local_files_succeed() {
    // 1. Cargo.toml relative to crate root
    let cargo_path = if std::path::Path::new("Cargo.toml").exists() {
        "Cargo.toml"
    } else {
        "src-tauri/Cargo.toml"
    };
    let res = validate_local_safe_path(cargo_path);
    assert!(
        res.is_ok(),
        "Valid file '{}' should be accepted, got error: {:?}",
        cargo_path,
        res
    );
    let canonical = res.unwrap();
    assert!(canonical.exists(), "Canonical path must exist on disk");
    assert!(!canonical.to_string_lossy().starts_with(r"\\"), "Canonical path must not be UNC");

    // 2. Existing src/lib.rs
    let lib_path = if std::path::Path::new("src/lib.rs").exists() {
        "src/lib.rs"
    } else {
        "src-tauri/src/lib.rs"
    };
    let res2 = validate_local_safe_path(lib_path);
    assert!(
        res2.is_ok(),
        "Valid file '{}' should be accepted, got error: {:?}",
        lib_path,
        res2
    );

    // 3. Dynamically created temp file with spaces and text
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join(format!("orbit_test_safe_valid_{}.txt", std::process::id()));
    {
        let mut f = File::create(&temp_file).expect("Failed to create temp file");
        writeln!(f, "Orbit safe content").unwrap();
    }

    let res_temp = validate_local_safe_path(&temp_file.to_string_lossy());
    assert!(
        res_temp.is_ok(),
        "Dynamically created file should be accepted, got: {:?}",
        res_temp
    );
    let canon_temp = res_temp.unwrap();
    assert!(canon_temp.exists());
    let raw_canon = temp_file.canonicalize().unwrap();
    let raw_canon_str = raw_canon.to_string_lossy();
    let expected = if raw_canon_str.starts_with(r"\\?\") {
        PathBuf::from(&raw_canon_str[4..])
    } else {
        raw_canon
    };
    assert_eq!(canon_temp, expected);

    let _ = fs::remove_file(&temp_file);
}

#[test]
fn test_shelf_executable_blocking_all_extensions() {
    let blocked_extensions = [
        "exe", "bat", "cmd", "msi", "vbs", "ps1", "com", "scr", "pif", "reg",
        "hta", "cpl", "jar", "wsf", "vbe", "jse",
    ];

    // Verify constant contains all required extensions
    for ext in blocked_extensions {
        assert!(
            DANGEROUS_EXTENSIONS.contains(&ext),
            "DANGEROUS_EXTENSIONS missing extension: '{}'",
            ext
        );
    }

    // Verify open_shelf_file execution rejection for each dangerous extension
    let db = DatabaseManager::in_memory().expect("in-memory db");
    let temp_dir = std::env::temp_dir();

    for ext in blocked_extensions {
        let uppercase_ext = ext.to_ascii_uppercase();
        for current_ext in [ext.to_string(), uppercase_ext] {
            // Clean shelf before each item to avoid 10-item limit
            db.clear_shelf().expect("clear shelf");

            let filename = format!("orbit_malicious_test_{}_{}.{}", std::process::id(), ext, current_ext);
            let filepath = temp_dir.join(&filename);
            {
                let mut f = File::create(&filepath).expect("Failed to create test file");
                writeln!(f, "echo attack").unwrap();
            }

            let add_res = db.add_shelf_item_with_mode(&filepath.to_string_lossy(), "reference");
            assert!(
                add_res.is_ok(),
                "File should be added to shelf metadata: {:?}",
                add_res
            );
            let item = add_res.unwrap();

            // Now call open_shelf_file and verify it is BLOCKED
            let open_res = db.open_shelf_file(&item.id);
            assert!(
                open_res.is_err(),
                "open_shelf_file should BLOCK dangerous extension '.{}', but got Ok(())!",
                current_ext
            );
            let err_msg = open_res.unwrap_err();
            assert!(
                err_msg.contains("blocked for security reasons"),
                "Expected security block error, got: '{}'",
                err_msg
            );

            let _ = fs::remove_file(&filepath);
        }
    }
}

#[test]
fn test_window_label_acl_strictly_enforced() {
    // 1. "settings" MUST be permitted
    let settings_res = verify_settings_window("settings");
    assert!(settings_res.is_ok(), "Window 'settings' must be allowed");

    // 2. Window labels that MUST be rejected
    let rejected_labels = [
        "main",
        "island",
        "SETTINGS", // Case sensitivity check
        "Settings",
        "settings ", // Trailing space check
        " settings",
        "settings\0",
        "overlay",
        "admin",
        "webview",
        "external",
        "",
        "unknown_window_id",
        "settings_clone",
    ];

    for label in rejected_labels {
        let res = verify_settings_window(label);
        assert!(
            res.is_err(),
            "Window label '{}' should be REJECTED, but was accepted!",
            label
        );
        let err = res.unwrap_err();
        assert_eq!(
            err, "Access denied: command only permitted from settings window",
            "Error message mismatch for label '{}': got '{}'",
            label, err
        );
    }
}

#[test]
fn test_alternate_data_streams_and_nonexistent_paths() {
    // Non-existent file must fail existence check
    assert!(validate_local_safe_path(r"C:\non_existent_orbit_123456.txt").is_err());

    // Alternate Data Stream referencing nonexistent stream or invalid path
    assert!(validate_local_safe_path(r"C:\Windows\System32\cmd.exe:hidden.exe").is_err());
}

// Copyright (c) 2026 The Nook contributors
// Licensed under the MIT License (see egui-app/THIRD_PARTY_NOTICES.md).
// Adapted from nook/crates/app/build.rs.

//! Embeds the ORBIT app icon and Windows version info into orbit-egui.exe.

fn main() {
    const ICON: &str = "../../assets/icon.ico";
    println!("cargo:rerun-if-changed={ICON}");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon(ICON)
        .set("ProductName", "ORBIT")
        .set("FileDescription", "ORBIT Dynamic Island")
        .set("LegalCopyright", "MIT License");
    res.compile().expect("embedding the icon resource");
}

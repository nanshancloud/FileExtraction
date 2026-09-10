fn main() {
    // Embed the application icon (plus basic version info) into the Windows
    // executable, so Explorer, the taskbar and Alt+Tab show the logo for the
    // built binary itself - not only for the running window.
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=assets/app.ico");

        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/app.ico");
        res.set("ProductName", "FileExtraction");
        res.set("FileDescription", "File Extraction Tool");
        res.set("OriginalFilename", "FileExtraction.exe");
        res.set("LegalCopyright", "power by Yang Haijun (LN1217)");

        // Never fail the build because of missing resource tooling
        if let Err(e) = res.compile() {
            println!("cargo:warning=failed to embed the Windows icon resource: {e}");
        }
    }
}

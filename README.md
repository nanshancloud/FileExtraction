# FileExtraction

A lightweight desktop tool for scanning directories by file extension and exporting the matching files together with an Excel inventory list.

Built with Rust + [eframe/egui](https://github.com/emilk/egui) — a single native executable, no installer, no runtime dependencies.

[中文文档](README_zh.md)

## Features

- **Recursive directory scan** — pick any folder and recursively collect files matching the selected extensions. Scanning runs on a background thread so the UI stays responsive, with a live progress counter.
- **Extension categories** — built-in groups:
  - *Default formats*: `doc`, `docx`, `ppt`, `pptx`, `xls`, `xlsx`, `txt`, `pdf`, `md`
  - *Report templates*: `det2app`
- **Custom extensions** — add your own extensions to a category (dots and case are normalized); they are validated, de-duplicated and persisted. Select all / deselect all per category with a hover-revealed button.
- **Real-time search** — filter the result table by file name as you type.
- **Interactive result table** — columns for No., file name, full path and human-readable size.
  - Double-click a **file name** to open it with the system default program.
  - Double-click a **file path** to open the containing folder with the file selected.
- **Export** — choose an output directory and:
  - copy all matched documents there, either flat or keeping the original folder structure;
  - name conflicts are resolved automatically with `name (2).ext` style suffixes;
  - a per-run summary is reported in the status bar (copied / total / failures).
- **Excel list generation** — an `.xlsx` inventory (`FileScanList.xlsx` / `文件扫描清单.xlsx`, localized per language) is written next to the exported files, containing No., file name, path, size in bytes and formatted size. The header row is styled and frozen.
- **Multi-language UI** — English, 简体中文, 繁體中文, 日本語, Español, Français. On first launch a language picker dialog is shown; the language can be switched at any time from the side panel and is applied live (including the window title).
- **Persistent settings** — selected extensions, custom extensions, language and the structure option are saved to `config.json` automatically. Legacy flat config files are migrated transparently.
- **CJK font support** — a CJK system font is auto-detected and registered as a fallback, so Chinese/Japanese text renders correctly out of the box.
- **Native look & feel** — embedded application icon shown in Explorer, the taskbar and Alt+Tab (Windows).

## Screenshots

> Add your own screenshots under `assets/` and reference them here, e.g.:
> ```markdown
> ![Main window](assets/screenshot.png)
> ```

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) toolchain (edition 2024, so Rust 1.85+)
- Windows (primary target; the icon resource step is Windows-only, other platforms build without it)

## Build & Run

```bash
# Run in development
cargo run

# Build an optimized release binary
cargo build --release
```

The release executable is produced at `target/release/FileExtraction.exe` (`.exe` on Windows). It is self-contained — just copy it anywhere and run.

## Usage

1. Launch the app and choose your language on first run.
2. In the **File Format Settings** panel on the left, check the extensions you want to scan. Add custom extensions if needed.
3. Use **Browse...** to select the directory to scan, then click **Start Scan**.
4. Review the results in the table; use the search box (top right) to filter by name.
5. Optionally enable **Export with original folder structure**.
6. Click **Export All Files**, pick an output folder, and the app copies the documents and generates the Excel list there.

## Configuration

Settings are stored in `config.json` in the same directory as the executable:

```json
{
  "custom_extensions": {
    "cat_default": ["csv"],
    "cat_report_template": []
  },
  "selected_extensions": ["doc", "docx", "pdf", "csv"],
  "keep_structure": false,
  "language": "en"
}
```

Deleting the file resets the app to defaults and shows the language picker again.

## Project Structure

```
src/
  main.rs       Application entry point, window/viewport setup
  app.rs        UI state and the egui interface (egui/eframe::App)
  config.rs     Extension categories, config load/save and migration
  scanner.rs    Background directory scan (walkdir) with pollable state
  exporter.rs   File copying (flat / structured) and Excel generation
  i18n.rs       Language enum, UI strings and the "{}" template formatter
  fonts.rs      CJK system font detection and registration
  icon.rs       Embedded PNG logo decoding for the window/UI icon
assets/
  app.ico       Windows executable icon
  logo.png      Application logo (also embedded in the binary)
build.rs        Embeds the Windows icon and version metadata
```

## Tech Stack

| Crate | Purpose |
| --- | --- |
| `eframe` / `egui` | Native GUI framework |
| `egui_extras` | Table widget for the result list |
| `rfd` | Native folder picker dialogs |
| `walkdir` | Recursive directory traversal |
| `rust_xlsxwriter` | Excel (`.xlsx`) list generation |
| `serde` / `serde_json` | Configuration serialization |
| `png` | Decoding the embedded logo |
| `winresource` | Embedding the Windows icon and metadata |

## License

Released under the MIT License. See `LICENSE` for details.

---

Copyright © Eric Yeung (LN1217 / nanshancloud)

# FileExtraction

A lightweight desktop tool for scanning directories by file extension and exporting the matching files together with an Excel inventory list.

Built with Rust + [eframe/egui](https://github.com/emilk/egui) — a single native executable, no installer, no runtime dependencies.

[中文文档](README_zh.md)

## Features

- **Recursive scan** — pick a folder and recursively collect files matching the selected extensions. The scan runs on a background thread with a live progress counter.
- **Extension categories** — built-in groups: *Default formats* (`doc`, `docx`, `ppt`, `pptx`, `xls`, `xlsx`, `txt`, `pdf`, `md`) and *Report templates* (`det2app`).
- **Custom extensions** — add your own extensions to a category; they are validated, de-duplicated and saved. Select all / deselect all per category.
- **Real-time search** — filter the result table by file name as you type.
- **Interactive table** — double-click a file name to open it, or a path to open its containing folder.
- **Export** — copy matched documents to an output folder, flat or keeping the original folder structure, with automatic renaming on conflicts.
- **Excel list** — generates an `.xlsx` inventory (No., name, path, size) in the export folder.
- **Multi-language UI** — English, 简体中文, 繁體中文, 日本語, Español, Français, switchable at any time.
- **Persistent settings** — selected and custom extensions, language and options are saved to `config.json` automatically.

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) 1.85+ (edition 2024)
- Windows (primary target; the icon embedding step is Windows-only)

## Build & Run

```bash
cargo run                 # development
cargo build --release     # optimized binary in target/release/
```

## Usage

1. Choose your language on first run.
2. In the **File Format Settings** panel, check the extensions to scan and add custom ones if needed.
3. Pick the directory to scan and click **Start Scan**.
4. Review results in the table; use the search box to filter by name.
5. Optionally enable **Export with original folder structure**.
6. Click **Export All Files**, choose an output folder, and the documents plus the Excel list are written there.

## Configuration

Settings are stored in `config.json` next to the executable:

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

Deleting the file resets the app and shows the language picker again.

## License

Released under the MIT License. See `LICENSE` for details.

---

Copyright © Eric Yeung

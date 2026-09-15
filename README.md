# FileExtraction

A lightweight desktop tool for scanning directories by file extension and exporting the matching files together with an Excel inventory list.

Built with Rust + [eframe/egui](https://github.com/emilk/egui) — a single native executable, no installer, no runtime dependencies.

[中文文档](README_zh.md)

## Features

- **Recursive scan** — pick a folder and collect the matching files in a background thread with a live progress counter.
- **Extension categories** — *Default formats* (`doc`, `docx`, `ppt`, `pptx`, `xls`, `xlsx`, `txt`, `pdf`, `md`), *Report templates* (`det2app`), *Archives* (`7z`, `rar`, `zip`, `tar`, `tar.gz`, `tar.xz`). Custom extensions can be added per category (compound ones such as `tar.bz2` are supported), with select all / deselect all.
- **Result table** — filter by name as you type; results are listed in directory-tree order (root files first, then sub-directory by sub-directory); double-click a header to sort (A-Z → Z-A → default, with a ▲ / ▼ marker); drag a separator to resize a column (double-click to auto-fit, widths are saved); double-click a file name to open it, or a path to open its folder. Each file also shows its last modified time.
- **Export** — copy the matched files to an output folder, flat or keeping the original structure with automatic renaming on conflicts, or generate the Excel list only.
- **Excel list** — an `.xlsx` inventory sorted A-Z by file name, with directory level columns and the data columns No., name, path, size, modified (raw bytes optional).
- **Settings** — language, size unit (Auto, or a fixed B / KB / MB / GB / TB), path display (absolute, or relative to the scan root shown as `/`), and column management (reorder / hide columns of the table and the list).
- **Remember last directory** — optionally restore the directory of the last scan and rescan it on startup.
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

1. Choose your language on first run (switch it later in **Settings**).
2. In the **File Format Settings** panel, check the extensions to scan and add custom ones if needed.
3. Pick the directory to scan and click **Start Scan**.
4. Review results in the table; use the search box to filter by name.
5. Optionally enable **Export with original folder structure**.
6. Click **Export All Files** to copy the documents and write the Excel list, or **Export List Only** to just generate the list.
7. Open **Settings** to change the language, or to reorder / hide the columns of the table and the list.

## Configuration

Settings are stored in `config.json` next to the executable:

```json
{
  "custom_extensions": {
    "cat_default": ["csv"],
    "cat_report_template": [],
    "cat_archive": []
  },
  "selected_extensions": ["doc", "docx", "pdf", "csv", "zip", "tar.gz"],
  "keep_structure": false,
  "language": "en",
  "columns": [
    { "id": "no", "visible": true },
    { "id": "name", "visible": true },
    { "id": "path", "visible": true },
    { "id": "size", "visible": true },
    { "id": "modified", "visible": true },
    { "id": "bytes", "visible": false, "width": null }
  ],
  "open_last_dir": true,
  "last_dir": "",
  "size_unit": "auto",
  "path_mode": "absolute"
}
```

`path_mode` is `absolute` (full path) or `relative` (from the scan root, which is shown as `/`); older config files without the key simply use `absolute`.

`columns` controls the order and the visibility of the result table and of the Excel list (`no`, `name`, `path`, `size`, `modified`, `bytes`). Missing entries fall back to the default order.

Deleting the file resets the app and shows the language picker again.

## License

Released under the MIT License. See `LICENSE` for details.

---

Copyright © Eric Yeung

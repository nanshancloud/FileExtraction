# FileExtraction

A lightweight desktop tool for scanning directories by file extension and exporting the matching files together with an Excel inventory list.

Built with Rust + [eframe/egui](https://github.com/emilk/egui) — a single native executable, no installer, no runtime dependencies.

[中文文档](README_zh.md)

## Features

- **Recursive scan** — pick a folder and recursively collect files matching the selected extensions. The scan runs on a background thread with a live progress counter.
- **Extension categories** — built-in groups: *Default formats* (`doc`, `docx`, `ppt`, `pptx`, `xls`, `xlsx`, `txt`, `pdf`, `md`), *Report templates* (`det2app`) and *Archives* (`7z`, `rar`, `zip`, `tar`, `tar.gz`, `tar.xz`).
- **Custom extensions** — add your own extensions to a category; they are validated, de-duplicated and saved. Compound extensions such as `tar.bz2` are supported. Select all / deselect all per category.
- **Real-time search** — filter the result table by file name as you type.
- **Interactive table** — double-click a file name to open it, or a path to open its containing folder.
- **Tree ordering** — results are listed the way a directory tree is read: root files first (A-Z), then sub-directory by sub-directory (A-Z), with the files of a directory always ahead of its sub-directories.
- **Last updated time** — every file shows its last update time (local time) in the table and in the list.
- **Export** — copy matched documents to an output folder, flat or keeping the original folder structure, with automatic renaming on conflicts.
- **Export list only** — generate the Excel inventory without copying any file.
- **Excel list** — generates an `.xlsx` inventory in the export folder. Directory level columns (`1st level` … `Nth level`) are created automatically from the real folder depth, and each directory name is written once and merged over the rows it covers. Data columns: No., name, path (absolute or relative, following the setting), size, last modified (and raw bytes, optional); rows are sorted A-Z by file name. Levels a file does not reach are filled with a centered `/` placeholder.
- **File size unit** — *Auto* (default) switches B / KB / MB / GB / TB per file size; a fixed unit can be selected in the settings and is then shown in the column header.
- **Remember last directory** — with *Open last used directory on startup* enabled, the app restores the directory of the last scan and rescans it, bringing the previous results back immediately.
- **Resizable columns** — drag any column separator to resize it (double-click the separator to auto-fit the content); the widths are saved with the config. Narrow columns wrap their text and the row grows accordingly.
- **Sort by header** — double-click a column header to sort by it: first A-Z, then Z-A, then back to the default order (root files first, then sub-directory by sub-directory). A ▲ / ▼ marker shows the sorted column and direction. Sorting only rearranges the results table — **the exported Excel list is always written A-Z by file name**.
- **Path display switch** — the settings offer *Absolute* / *Relative*, absolute by default. Relative paths start at the scan root: the root itself is `/` and every level below it is joined with `/` (e.g. `/sub folder/report.pdf`). The result table and the exported list follow the switch together; hovering a row reveals the other variant and double-click still opens the containing folder.
- **Settings** — language switch, file size unit, path display (absolute / relative), and column management: reorder and hide columns of the result table and the Excel list.
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

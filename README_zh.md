# FileExtraction 文件提取工具

一款轻量级桌面工具，用于按文件扩展名扫描目录，并将匹配的文件连同 Excel 清单一起导出。

基于 Rust + [eframe/egui](https://github.com/emilk/egui) 构建 —— 单个原生可执行文件，无需安装、无运行时依赖。

[English](README.md)

## 功能特性

- **递归扫描** —— 选择文件夹后递归收集符合所选扩展名的文件，扫描在后台线程中进行并实时显示进度。
- **扩展名分类** —— 内置分组：*默认格式*（`doc`、`docx`、`ppt`、`pptx`、`xls`、`xlsx`、`txt`、`pdf`、`md`）、*报告模板*（`det2app`）与 *压缩文件*（`7z`、`rar`、`zip`、`tar`、`tar.gz`、`tar.xz`）；可向分类中添加自定义扩展名（支持 `tar.bz2` 这类复合扩展名），并支持全选 / 反选。
- **结果表格** —— 输入即按文件名过滤；结果按目录树顺序排列（根目录文件优先，随后逐级子目录）；双击列头排序（A-Z → Z-A → 默认，列头以 ▲ / ▼ 标示）；拖动分隔线调整列宽（双击自动适配，宽度随配置保存）；双击文件名打开文件、双击路径打开所在目录；并显示文件最后修改时间。
- **导出** —— 将匹配文件复制到输出目录，支持平铺或保留原目录结构、重名自动重命名；也可「仅导出清单」。
- **Excel 清单** —— 生成按文件名 A-Z 排序的 `.xlsx` 清单，含目录层级列与数据列：序号、名称、路径、大小、修改时间（可选字节数）。
- **设置** —— 语言、文件大小单位（自动，或固定 B / KB / MB / GB / TB）、路径显示（绝对路径，或以扫描根目录为 `/` 的相对路径）、列管理（调整表格与清单的列顺序、隐藏列）。
- **记住上次目录** —— 可开启启动时恢复上次扫描目录并自动重新扫描。
- **多语言界面** —— 支持 English、简体中文、繁體中文、日本語、Español、Français，可随时切换。
- **配置持久化** —— 所选扩展名、自定义扩展名、语言与选项自动保存到 `config.json`。

## 环境要求

- [Rust](https://www.rust-lang.org/tools/install) 1.85+（edition 2024）
- Windows（主要目标平台；图标嵌入步骤为 Windows 专属）

## 构建与运行

```bash
cargo run                 # 开发模式运行
cargo build --release     # 生成发布版，位于 target/release/
```

## 使用步骤

1. 首次启动时选择语言（之后可在 **设置** 中切换）。
2. 在 **文件格式配置** 面板中勾选需要扫描的扩展名，可按需添加自定义扩展名。
3. 选择待扫描目录并点击 **开始扫描**。
4. 在表格中查看结果，使用搜索框按文件名过滤。
5. 如需保留目录结构，勾选 **按原目录结构导出**。
6. 点击 **导出所有文件** 复制文档并生成清单，或点击 **仅导出清单** 只生成清单。
7. 点击 **设置** 可切换语言，或调整表格与清单的列顺序、显示 / 隐藏列。

## 配置文件

配置保存在可执行文件同目录下的 `config.json`：

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
  "size_unit": "auto"
}
```

`columns` 控制结果表格与 Excel 清单的列顺序和显示状态（`no`、`name`、`path`、`size`、`modified`、`bytes`）；缺少的列会自动按默认顺序补齐。

删除该文件即可重置为默认设置，并重新弹出语言选择。

## 许可证

基于 MIT License 发布，详见 `LICENSE`。

---

版权所有 © Eric Yeung

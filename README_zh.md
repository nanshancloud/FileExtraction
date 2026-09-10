# FileExtraction 文件提取工具

一款轻量级桌面工具，用于按文件扩展名扫描目录，并将匹配的文件连同 Excel 清单一起导出。

基于 Rust + [eframe/egui](https://github.com/emilk/egui) 构建 —— 单个原生可执行文件，无需安装、无运行时依赖。

[English](README.md)

## 功能特性

- **递归目录扫描** —— 选择任意文件夹，递归收集符合所选扩展名的文件。扫描在后台线程中进行，界面保持流畅，并实时显示进度计数。
- **扩展名分类** —— 内置分组：
  - *默认格式*：`doc`、`docx`、`ppt`、`pptx`、`xls`、`xlsx`、`txt`、`pdf`、`md`
  - *报告模板*：`det2app`
- **自定义扩展名** —— 可向分类中添加自己的扩展名（自动忽略大小写与前导点），会进行校验、去重并持久化保存。每个分类支持全选 / 反选（悬停时显示按钮）。
- **实时搜索** —— 输入即可按文件名过滤结果表格。
- **可交互结果表格** —— 包含序号、文件名称、完整路径、易读的文件大小四列。
  - 双击**文件名称**用系统默认程序打开文件。
  - 双击**文件路径**打开所在目录并定位到该文件。
- **导出功能** —— 选择输出目录后：
  - 将匹配的文档复制过去，支持平铺或保留原始目录结构；
  - 重名时自动以 `名称 (2).扩展名` 形式重命名；
  - 状态栏显示本次导出结果摘要（成功 / 总数 / 失败数）。
- **Excel 清单生成** —— 在导出目录下生成 `.xlsx` 清单（`FileScanList.xlsx` / `文件扫描清单.xlsx`，随语言本地化），包含序号、文件名称、路径、字节大小与格式化大小。表头带样式并冻结首行。
- **多语言界面** —— 支持 English、简体中文、繁體中文、日本語、Español、Français。首次启动弹出语言选择对话框；之后可随时在侧边栏切换，界面（含窗口标题）即时更新。
- **配置持久化** —— 所选扩展名、自定义扩展名、语言与结构选项自动保存到 `config.json`，旧版平铺配置可无缝迁移。
- **中日韩字体支持** —— 自动检测并注册系统中文字体作为回退，中文/日文可开箱正确显示。
- **原生观感** —— 内嵌应用图标，在资源管理器、任务栏与 Alt+Tab 中均正常显示（Windows）。

## 界面截图

> 可将截图放入 `assets/` 目录并在此引用，例如：
> ```markdown
> ![主界面](assets/screenshot.png)
> ```

## 环境要求

- [Rust](https://www.rust-lang.org/tools/install) 工具链（使用 edition 2024，需 Rust 1.85+）
- Windows（主要目标平台；图标资源嵌入为 Windows 专属步骤，其他平台可正常编译）

## 构建与运行

```bash
# 开发模式运行
cargo run

# 构建发布版可执行文件
cargo build --release
```

发布版可执行文件位于 `target/release/FileExtraction.exe`（Windows）。它是自包含的，复制到任意位置即可运行。

## 使用步骤

1. 启动程序，首次运行时选择语言。
2. 在左侧 **文件格式配置** 面板中勾选需要扫描的扩展名，可按需添加自定义扩展名。
3. 点击 **选择目录...** 选择待扫描目录，然后点击 **开始扫描**。
4. 在表格中查看结果；使用右上角的搜索框按文件名过滤。
5. 如需保留目录结构，勾选 **按原目录结构导出**。
6. 点击 **导出所有文件**，选择输出目录，程序将复制文档并在该目录生成 Excel 清单。

## 配置文件

配置保存在可执行文件同目录下的 `config.json`：

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

删除该文件即可重置为默认设置，并重新弹出语言选择。

## 项目结构

```
src/
  main.rs       程序入口，窗口/视口初始化
  app.rs        UI 状态与 egui 界面（egui/eframe::App）
  config.rs     扩展名分类、配置读写与迁移
  scanner.rs    后台目录扫描（walkdir），提供可轮询状态
  exporter.rs   文件复制（平铺 / 保留结构）与 Excel 生成
  i18n.rs       语言枚举、界面文案与 "{}" 模板格式化
  fonts.rs      系统中文字体检测与注册
  icon.rs       内嵌 PNG 图标解码（窗口与界面图标）
assets/
  app.ico       Windows 可执行文件图标
  logo.png      应用 Logo（同时内嵌进二进制）
build.rs        嵌入 Windows 图标与版本信息
```

## 技术栈

| 依赖 | 用途 |
| --- | --- |
| `eframe` / `egui` | 原生 GUI 框架 |
| `egui_extras` | 结果列表的表格控件 |
| `rfd` | 原生目录选择对话框 |
| `walkdir` | 递归目录遍历 |
| `rust_xlsxwriter` | Excel（`.xlsx`）清单生成 |
| `serde` / `serde_json` | 配置序列化 |
| `png` | 内嵌 Logo 解码 |
| `winresource` | 嵌入 Windows 图标与元信息 |

## 许可证

基于 MIT License 发布，详见 `LICENSE`。

---

版权所有 © Eric Yeung (LN1217 / nanshancloud)

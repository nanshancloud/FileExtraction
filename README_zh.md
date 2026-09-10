# FileExtraction 文件提取工具

一款轻量级桌面工具，用于按文件扩展名扫描目录，并将匹配的文件连同 Excel 清单一起导出。

基于 Rust + [eframe/egui](https://github.com/emilk/egui) 构建 —— 单个原生可执行文件，无需安装、无运行时依赖。

[English](README.md)

## 功能特性

- **递归扫描** —— 选择文件夹后递归收集符合所选扩展名的文件，扫描在后台线程中进行并实时显示进度。
- **扩展名分类** —— 内置分组：*默认格式*（`doc`、`docx`、`ppt`、`pptx`、`xls`、`xlsx`、`txt`、`pdf`、`md`）与 *报告模板*（`det2app`）。
- **自定义扩展名** —— 可向分类中添加扩展名，自动校验、去重并保存；每个分类支持全选 / 反选。
- **实时搜索** —— 输入即可按文件名过滤结果表格。
- **可交互表格** —— 双击文件名打开文件，双击路径打开所在目录。
- **导出** —— 将匹配文档复制到输出目录，支持平铺或保留原目录结构，重名自动重命名。
- **Excel 清单** —— 在导出目录生成 `.xlsx` 清单（序号、名称、路径、大小）。
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

1. 首次启动时选择语言。
2. 在 **文件格式配置** 面板中勾选需要扫描的扩展名，可按需添加自定义扩展名。
3. 选择待扫描目录并点击 **开始扫描**。
4. 在表格中查看结果，使用搜索框按文件名过滤。
5. 如需保留目录结构，勾选 **按原目录结构导出**。
6. 点击 **导出所有文件**，选择输出目录，文档与 Excel 清单将输出到该目录。

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

## 许可证

基于 MIT License 发布，详见 `LICENSE`。

---

版权所有 © Eric Yeung

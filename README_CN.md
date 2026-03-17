# Tauri 跨平台模板

一个全面的模板，用于构建支持 Windows 7、10、11 和 macOS 的跨平台桌面应用程序。本模板演示了如何配置 Tauri 与 React 和 TypeScript，同时保持与传统和现代操作系统的兼容性。

**English Documentation**: [README.md](./README.md)

## 🚀 特性

- **跨平台支持**：完全兼容 Windows 7、10、11 和 macOS
- **现代技术栈**：基于最新的 Tauri、React 18、TypeScript 和 Vite 构建
- **固定 WebView2 运行时**：为传统系统离线安装嵌入的 WebView2 运行时
- **优化构建配置**：为传统和现代平台预配置设置
- **生产就绪**：包含构建可分发应用程序的所有必要配置

## 📋 先决条件

- **Windows**：Windows 7 SP1 或更高版本
- **macOS**：macOS 10.13 或更高版本
- [Node.js](https://nodejs.org/)（v16 或更高版本）
- [Rust](https://rustup.rs/) 工具链
- [pnpm](https://pnpm.io/) 包管理器（推荐）

## 🛠️ 推荐的 IDE 设置

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## 📦 安装

1. 克隆此仓库：
```bash
git clone https://github.com/your-username/tauri-cross-platform-template.git
cd tauri-cross-platform-template
```

2. 安装依赖：
```bash
pnpm install
```

## 🔧 平台配置

### Windows 7 用户

如果需要支持 Windows 7，请按照以下额外步骤操作：

#### 步骤 1：降级 Rust 工具链

Windows 7 需要 Rust 1.77.2 或更早版本（支持 Windows 7 的最后一个版本）：

```bash
rustup install 1.77.2
rustup default 1.77.2
rustc --version  # 应显示 1.77.2
```

该存储库还包含 `rust-toolchain.toml`，因此当您在此项目中运行 Rust 命令时，会自动选择 Rust 1.77.2。

#### 步骤 2：下载 WebView2 运行时（手动）

下载支持 Windows 7 的固定 WebView2 运行时版本：
- [WebView2 Runtime Archive](https://github.com/westinyang/WebView2RuntimeArchive/releases/tag/109.0.1518.78)

下载以下文件：
- `Microsoft.WebView2.FixedVersionRuntime.109.0.1518.78.x64.cab`

#### 步骤 3：提取运行时到项目

将运行时文件提取到项目的 `src-tauri` 目录：

```powershell
Expand .\Microsoft.WebView2.FixedVersionRuntime.109.0.1518.78.x64.cab -F:* ./src-tauri
```

#### 步骤 4：构建依赖

构建原生依赖以解决兼容性问题：

```bash
cd src-tauri
cargo build
```

**注意**：如果遇到依赖版本冲突，您可能需要手动降级需要更新 Rust 版本的 crate。找到与 Rust 1.77.2 兼容的版本。

### 预配置功能

此模板已包含：

- **WebView2 运行时**：嵌入固定版本（109.0.1518.78）以支持 Windows 7 兼容性
- **Tauri 配置**：在 `tauri.conf.json` 中配置了固定运行时路径
- **兼容性依赖**：预选择了 Windows 7 兼容版本
- **构建设置**：为跨版本 Windows 支持优化的配置

### Windows 10/11 用户

无需额外配置！模板在现代 Windows 版本上开箱即用。

### macOS 用户

无需额外配置！模板在所有支持的 macOS 版本上开箱即用。只需安装依赖并运行：

```bash
pnpm install
pnpm tauri dev  # 开发模式
pnpm tauri build  # 生产构建
```

## 🏗️ 构建应用程序

### 开发模式

```bash
pnpm tauri dev
```

### 生产构建

```bash
pnpm tauri build
```

构建的应用程序将在 `src-tauri/target/release/bundle/` 中可用。

## 📁 项目结构

```
├── src/                    # 前端源代码（React + TypeScript）
├── src-tauri/             # Tauri 后端源代码（Rust）
│   ├── src/               # Rust 源文件
│   ├── Cargo.toml         # Rust 依赖
│   ├── tauri.conf.json    # Tauri 配置
│   └── icons/             # 应用程序图标
├── public/                # 静态资源
└── dist/                  # 构建输出
```

## ⚙️ 配置

### Tauri 配置

主要配置位于 `src-tauri/tauri.conf.json`。Windows 7 兼容性的关键设置：

- `webviewInstallMode`：设置为使用固定运行时
- `bundle.windows.certificate`：代码签名（可选）
- `security`：内容安全策略

### 前端配置

- `vite.config.ts`：Vite 构建配置
- `tsconfig.json`：TypeScript 编译器选项
- `package.json`：Node.js 依赖和脚本

## 🐛 故障排除

### 常见问题

1. **Windows 7 上构建失败**：确保您使用的是 Rust 1.77.2 和兼容的依赖版本
2. **WebView2 运行时错误**：运行时文件已预包含在此模板中
3. **权限问题**：首次设置时如需要，请以管理员身份运行
4. **现代 Windows 兼容性**：此模板在 Windows 10 和 11 上无缝工作

### 依赖降级（仅限 Windows 7）

如果在 Windows 7 上遇到 Rust 版本冲突，请检查每个依赖的文档以获取兼容版本。可能需要降级的常见软件包：
- `tokio`：使用与 Rust 1.77.2 兼容的版本
- `serde`：确保版本兼容性
- `tauri`：使用支持 Windows 7 的 2.x 版本

## 🤝 贡献

欢迎贡献！请随时提交 Pull Request。

## 📄 许可证

本项目在 MIT 许可证下授权 - 详见 [LICENSE](LICENSE) 文件。

## 🙏 致谢

- [Tauri](https://tauri.app/) 提供了出色的框架
- [WebView2RuntimeArchive](https://github.com/westinyang/WebView2RuntimeArchive) 由 [westinyang](https://github.com/westinyang) 提供 Windows 7 兼容的 WebView2 运行时
- Rust 和 React 社区提供了优秀的工具和库

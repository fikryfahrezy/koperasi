# Tauri Cross-Platform Template

A comprehensive template for building cross-platform desktop applications using Tauri with full Windows 7, 10, 11, and macOS support. This template demonstrates how to configure Tauri with React and TypeScript while maintaining compatibility with both legacy and modern operating systems.

**中文文档**: [README_CN.md](./README_CN.md)

## 🚀 Features

- **Cross-Platform Support**: Full compatibility with Windows 7, 10, 11, and macOS
- **Modern Tech Stack**: Built with latest Tauri, React 19, TypeScript, and Vite
- **Fixed WebView2 Runtime**: Embedded WebView2 runtime for offline installation on legacy Windows systems
- **Optimized Build Configuration**: Pre-configured settings for both legacy and modern platforms
- **Production Ready**: Includes all necessary configurations for building distributable applications

## 📋 Prerequisites

- **Windows**: Windows 7 SP1 or later
- **macOS**: macOS 10.13 or later
- [Node.js](https://nodejs.org/) (v16 or higher)
- [Rust](https://rustup.rs/) toolchain
- [pnpm](https://pnpm.io/) package manager (recommended)

## 🛠️ Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## 📦 Installation

1. Clone this repository:
```bash
git clone https://github.com/your-username/tauri-windows7-template.git
cd tauri-windows7-template
```

2. Install dependencies:
```bash
pnpm install
```

## 🔧 Platform Configuration

### For Windows 7 Users

If you need to support Windows 7, follow these additional steps:

#### Step 1: Downgrade Rust Toolchain

Windows 7 requires Rust 1.77.2 or earlier (the last version supporting Windows 7):

```bash
rustup install 1.77.2
rustup default 1.77.2
rustc --version  # Should show 1.77.2
```

This repository also includes `rust-toolchain.toml`, so Rust 1.77.2 is automatically selected when you run Rust commands in this project.

#### Step 2: Download WebView2 Runtime (Manual)

Download the fixed WebView2 runtime version that supports Windows 7:
- [WebView2 Runtime Archive](https://github.com/westinyang/WebView2RuntimeArchive/releases/tag/109.0.1518.78)

Download the following file:
- `Microsoft.WebView2.FixedVersionRuntime.109.0.1518.78.x64.cab`

#### Step 3: Extract Runtime to Project

Extract the runtime files to your project's `src-tauri` directory:

```powershell
Expand .\Microsoft.WebView2.FixedVersionRuntime.109.0.1518.78.x64.cab -F:* ./src-tauri
```

#### Step 4: Build Dependencies

Build native dependencies to resolve compatibility issues:

```bash
cd src-tauri
cargo build
```

**Note**: If you encounter dependency version conflicts, you may need to manually downgrade crates that require newer Rust versions.

### Pre-configured Features

This template already includes:

- **WebView2 Runtime**: Fixed version (109.0.1518.78) embedded for Windows 7 compatibility
- **Tauri Configuration**: Fixed runtime path configured in `tauri.conf.json`
- **Compatibility Dependencies**: Windows 7 compatible versions pre-selected
- **Build Settings**: Optimized configuration for cross-version Windows support

### For Windows 10/11 Users

No additional configuration required! The template works out-of-the-box with modern Windows versions.

## 🏗️ Building Your Application

### Development Mode

```bash
pnpm tauri dev
```

### Production Build

```bash
pnpm tauri build
```

The built application will be available in `src-tauri/target/release/bundle/`.

## 📁 Project Structure

```
├── src/                    # Frontend source code (React + TypeScript)
├── src-tauri/             # Tauri backend source code (Rust)
│   ├── src/               # Rust source files
│   ├── Cargo.toml         # Rust dependencies
│   ├── tauri.conf.json    # Tauri configuration
│   └── icons/             # Application icons
├── public/                # Static assets
└── dist/                  # Build output
```

## ⚙️ Configuration

### Tauri Configuration

The main configuration is located in `src-tauri/tauri.conf.json`. Key settings for Windows 7 compatibility:

- `webviewInstallMode`: Set to use fixed runtime
- `bundle.windows.certificate`: Optional for code signing
- `security`: CSP policies for content security

### Frontend Configuration

- `vite.config.ts`: Vite build configuration
- `tsconfig.json`: TypeScript compiler options
- `package.json`: Node.js dependencies and scripts

## 🐛 Troubleshooting

### Common Issues

1. **Build Fails on Windows 7**: Ensure you're using Rust 1.77.2 and compatible dependency versions
2. **WebView2 Runtime Errors**: Runtime files are pre-included in this template
3. **Permission Issues**: Run as administrator if needed during first-time setup
4. **Modern Windows Compatibility**: This template works seamlessly on Windows 10 and 11

### Dependency Downgrades (Windows 7 Only)

If you encounter Rust version conflicts on Windows 7, check each dependency's documentation for compatible versions. Common packages that may need downgrading:
- `tokio`: Use versions compatible with Rust 1.77.2
- `serde`: Ensure version compatibility
- `tauri`: Use version 2.x with Windows 7 support

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [Tauri](https://tauri.app/) for the amazing framework
- [WebView2RuntimeArchive](https://github.com/westinyang/WebView2RuntimeArchive) by [westinyang](https://github.com/westinyang) for providing Windows 7 compatible WebView2 runtimes
- The Rust and React communities for their excellent tools and libraries
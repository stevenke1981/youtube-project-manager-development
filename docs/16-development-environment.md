# Development Environment

## Windows 建議

- Windows 10 22H2 或 Windows 11。
- Visual Studio 2022 Build Tools：Desktop development with C++、Windows SDK。
- Rust stable（MSVC toolchain）。
- Node.js 22 LTS 或專案指定版本。
- WebView2 Runtime。
- Git。
- FFmpeg（進入 Media phase 才必要）。

## Commands

```powershell
rustup default stable-msvc
npm install
cargo test --workspace
npm run desktop:dev
```

## Version Policy

實際首次 bootstrap 後提交 `Cargo.lock` 與 `package-lock.json`。不要在 release branch 使用 floating latest。依賴升級要經 CI、migration fixture 與 Windows installer smoke test。

# Release & Packaging

## Channels

- dev：未簽署，內部。
- beta：給測試者，資料格式必須可 migration。
- stable：簽署安裝包。

## Windows

輸出 MSI／NSIS 擇一主方案。安裝程式只安裝 App，不建立或刪除 Library。解除安裝後保留使用者專案。

## Versioning

SemVer。Schema migration 另有版本。任何 downgrade 不保證自動支援，release notes 必須說明。

## Release Checklist

- cargo fmt／clippy／test。
- npm typecheck／test／build。
- E2E on clean Windows VM。
- upgrade from previous stable。
- uninstall preserves data。
- SBOM、licenses、checksums。
- code signing／malware scan。
- rollback installer retained。

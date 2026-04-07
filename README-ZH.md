<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/cover_dark.png">
  <source media="(prefers-color-scheme: light)" srcset="assets/cover_light.png">
  <img alt="OKX Plugin Store" src="assets/cover_dark.png" width="100%">
</picture>

[English](README.md) | [中文](README-ZH.md)

发现、安装和构建用于 DeFi、交易和 Web3 的 AI 代理Plugin。

**支持平台：** Claude Code、Cursor、OpenClaw

## 安装 Plugin Store

```bash
npx skills add MigOKG/plugin-store --skill plugin-store
```

将 Plugin Store 技能安装到你的 AI 代理中，实现Plugin发现和管理功能。

## 安装Plugin

```bash
# 安装指定Plugin
npx skills add MigOKG/plugin-store --skill <plugin-name>
```

---

## 贡献

提交 Plugin 请参阅开发者指南：[English](docs/FOR-DEVELOPERS.md) | [中文](docs/FOR-DEVELOPERS-ZH.md)

流程为 Fork 仓库、开发 Plugin，然后提交 Pull Request。

## 安全

如需报告安全问题，请发送邮件至 [security@okx.com](mailto:security@okx.com)。请勿就安全漏洞创建公开 Issue。

## 许可证

Apache-2.0

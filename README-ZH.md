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
npx skills add okx/plugin-store --skill plugin-store
```

将 Plugin Store 技能安装到你的 AI 代理中，实现Plugin发现和管理功能。

## 安装Plugin

```bash
# 安装指定Plugin
npx skills add okx/plugin-store --skill <plugin-name>
```

---

## 贡献

暂时不接受外部独立开发者的提交。我们会主动收录市场上流行的 Plugin。本仓库公开展示所有已收录 Plugin 的源码，供大家参考。技术参考：[English](docs/FOR-DEVELOPERS.md) | [中文](docs/FOR-DEVELOPERS-ZH.md)。

## 安全
如需报告安全问题，请通过我们的漏洞悬赏计划——HackerOne平台提交报告，网址为 [https://hackerone.com/okg?type=team](https://hackerone.com/okg?type=team)。请勿针对安全漏洞公开在 Github 上提交问题报告或 issues。

## 许可证

Apache-2.0

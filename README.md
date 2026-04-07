<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/cover_dark.png">
  <source media="(prefers-color-scheme: light)" srcset="assets/cover_light.png">
  <img alt="OKX Plugin Store" src="assets/cover_dark.png" width="100%">
</picture>

[English](README.md) | [中文](README-ZH.md)

Discover, install, and build AI agent plugins for DeFi, trading, and Web3.

**Supported platforms:** Claude Code, Cursor, OpenClaw

## Install Plugin Store

```bash
npx skills add okx/plugin-store --skill plugin-store
```

This installs the Plugin Store skill into your AI agent, enabling plugin discovery and management.

## Install a Plugin

```bash
# Browse all available plugins
npx skills add okx/plugin-store

# Install a specific plugin
npx skills add okx/plugin-store --skill <plugin-name>
```

---

## Documentation

| | English | 中文 |
|---|---------|------|
| Plugin Developer Guide | [FOR-DEVELOPERS.md](docs/FOR-DEVELOPERS.md) | [FOR-DEVELOPERS-ZH.md](docs/FOR-DEVELOPERS-ZH.md) |

## Contributing

To submit a plugin, see [FOR-DEVELOPERS.md](docs/FOR-DEVELOPERS.md) ([中文](docs/FOR-DEVELOPERS-ZH.md)). The workflow is Fork, develop, then open a Pull Request.

## Security

To report a security issue, please email [security@okx.com](mailto:security@okx.com). Do not open a public issue for security vulnerabilities.

## License

Apache-2.0

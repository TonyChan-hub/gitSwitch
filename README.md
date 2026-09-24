# GitSwitch

本地多仓库分支与 Git 身份（Profile / SSH）管理工具，基于 Tauri 2 + React。

## 功能（初版）

- 添加 / 切换多个本地 Git 仓库
- 按 remote Tab 查看、筛选、创建、切换、删除分支
- Fetch / Push（可用当前 Profile 的 SSH key）
- Profile：`user.name` / `user.email` / `core.sshCommand`
- 一键应用到当前仓库（写入 **local** git config）
- 配置文件：`~/.gitswitch/config.json`（若已有 `~/.gitbench` 会继续沿用）

## 要求

- Node.js 20+
- Rust（[rustup](https://rustup.rs/)）
- 系统已安装 `git`
- macOS：Xcode CLT（`xcode-select --install`）

## 开发

```bash
cd gitSwitch
npm install
source "$HOME/.cargo/env"
npm run tauri dev
```

## 构建

```bash
npm run tauri build
```

### CI 自动打包

推送到 `main`（或在 Actions 里手动 **Run workflow**）会自动构建：

- macOS Apple Silicon（`.dmg` / `.app`）
- macOS Intel
- Windows
- Linux

产物会同时：

1. **GitHub Releases**（推荐）：发布到 [Releases](https://github.com/TonyChan-hub/gitSwitch/releases) 标签 `v0.1.0`（随 `tauri.conf.json` 版本），安装包长期可下
2. **Actions Artifacts**：对应运行页的 Artifacts（约 90 天）

未配置 Apple 开发者证书时使用 ad-hoc 签名；本机首次打开若被拦截，可在「系统设置 → 隐私与安全性」里允许，或执行：

```bash
xattr -cr /path/to/GitSwitch.app
```

若 Release 上传报 `Resource not accessible by integration`，到仓库 **Settings → Actions → General → Workflow permissions** 勾选 **Read and write permissions**。

## 使用提示

1. 左侧「添加」选择带 `.git` 的目录
2. 右侧编辑 / 导入 Profile（邮箱 + 私钥）
3. 「应用到当前仓」后看顶栏身份与 SSH 状态
4. 建议配合 `~/.ssh/config` 的 Host 别名使用

不会改全局 `~/.gitconfig` 的 user，只写仓库级 local config。

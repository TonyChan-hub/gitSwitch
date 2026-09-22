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

## 使用提示

1. 左侧「添加」选择带 `.git` 的目录
2. 右侧编辑 / 导入 Profile（邮箱 + 私钥）
3. 「应用到当前仓」后看顶栏身份与 SSH 状态
4. 建议配合 `~/.ssh/config` 的 Host 别名使用

不会改全局 `~/.gitconfig` 的 user，只写仓库级 local config。

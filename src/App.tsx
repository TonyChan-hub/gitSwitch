import { useCallback, useEffect, useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api } from "./api";
import type { AppConfig, BranchInfo, Profile, RepoStatus, SshKeyInfo } from "./types";

type CenterTab = "branches" | "commits" | "remotes";
type BranchScope = "local" | string;

const emptyProfile = (): Profile => ({
  id: "",
  name: "",
  userName: "",
  userEmail: "",
  sshKeyPath: "",
  sshHostAlias: "",
});

function parseLogLine(line: string) {
  const [hash = "", subject = "", author = "", when = ""] = line.split("\t");
  return { hash, subject, author, when, raw: line };
}

export default function App() {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [branches, setBranches] = useState<BranchInfo[]>([]);
  const [status, setStatus] = useState<RepoStatus | null>(null);
  const [logs, setLogs] = useState<string[]>([]);
  const [sshKeys, setSshKeys] = useState<SshKeyInfo[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [newBranch, setNewBranch] = useState("");
  const [editing, setEditing] = useState<Profile | null>(null);
  const [filter, setFilter] = useState("");
  const [centerTab, setCenterTab] = useState<CenterTab>("branches");
  const [branchScope, setBranchScope] = useState<BranchScope>("local");

  const selectedRepo = useMemo(() => {
    if (!config?.selectedRepoId) return null;
    return config.repos.find((r) => r.id === config.selectedRepoId) ?? null;
  }, [config]);

  const boundProfile = useMemo(() => {
    if (!config || !selectedRepo?.profileId) return null;
    return config.profiles.find((p) => p.id === selectedRepo.profileId) ?? null;
  }, [config, selectedRepo]);

  const parsedLogs = useMemo(() => logs.map(parseLogLine), [logs]);

  const refreshRepo = useCallback(async (repoId: string) => {
    const [b, s, l] = await Promise.all([
      api.listBranches(repoId),
      api.getRepoStatus(repoId),
      api.recentLog(repoId, 30),
    ]);
    setBranches(b);
    setStatus(s);
    setLogs(l);
  }, []);

  const bootstrap = useCallback(async () => {
    setError(null);
    const [cfg, keys] = await Promise.all([api.getConfig(), api.listSshKeys()]);
    setConfig(cfg);
    setSshKeys(keys);
    if (cfg.selectedRepoId) {
      await refreshRepo(cfg.selectedRepoId);
    } else {
      setBranches([]);
      setStatus(null);
      setLogs([]);
    }
  }, [refreshRepo]);

  useEffect(() => {
    bootstrap().catch((e) => setError(String(e)));
  }, [bootstrap]);

  async function run(action: () => Promise<void>) {
    setBusy(true);
    setError(null);
    try {
      await action();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function onAddRepo() {
    await run(async () => {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "选择 Git 仓库目录",
      });
      if (!selected || Array.isArray(selected)) return;
      const cfg = await api.addRepo(selected);
      setConfig(cfg);
      if (cfg.selectedRepoId) await refreshRepo(cfg.selectedRepoId);
    });
  }

  async function onSelectRepo(id: string) {
    await run(async () => {
      const cfg = await api.selectRepo(id);
      setConfig(cfg);
      setCenterTab("branches");
      setBranchScope("local");
      setFilter("");
      await refreshRepo(id);
    });
  }

  async function onRemoveRepo(id: string) {
    if (!confirm("从列表移除该仓库？（不会删除磁盘文件）")) return;
    await run(async () => {
      const cfg = await api.removeRepo(id);
      setConfig(cfg);
      if (cfg.selectedRepoId) await refreshRepo(cfg.selectedRepoId);
      else {
        setBranches([]);
        setStatus(null);
        setLogs([]);
      }
    });
  }

  async function onCheckout(branch: string) {
    if (!selectedRepo) return;
    await run(async () => {
      await api.checkoutBranch(selectedRepo.id, branch);
      await refreshRepo(selectedRepo.id);
    });
  }

  async function onCreateBranch() {
    if (!selectedRepo || !newBranch.trim()) return;
    await run(async () => {
      await api.createBranch(selectedRepo.id, newBranch.trim(), true);
      setNewBranch("");
      await refreshRepo(selectedRepo.id);
    });
  }

  async function onDeleteBranch(name: string) {
    if (!selectedRepo) return;
    if (!confirm(`删除本地分支 ${name}？`)) return;
    await run(async () => {
      await api.deleteBranch(selectedRepo.id, name, false);
      await refreshRepo(selectedRepo.id);
    });
  }

  async function onApplyProfile(profileId: string) {
    if (!selectedRepo) return;
    await run(async () => {
      const s = await api.applyProfile(selectedRepo.id, profileId);
      setStatus(s);
      const cfg = await api.getConfig();
      setConfig(cfg);
    });
  }

  async function onSaveProfile() {
    if (!editing) return;
    await run(async () => {
      const cfg = await api.upsertProfile(editing);
      setConfig(cfg);
      setEditing(null);
    });
  }

  async function onImportSshKeys() {
    await run(async () => {
      const cfg = await api.importMissingSshProfiles();
      setConfig(cfg);
      setSshKeys(await api.listSshKeys());
    });
  }

  async function onDeleteProfile(id: string) {
    if (!confirm("删除此 Profile？")) return;
    await run(async () => {
      const cfg = await api.deleteProfile(id);
      setConfig(cfg);
    });
  }

  async function onFetch() {
    if (!selectedRepo) return;
    await run(async () => {
      await api.fetchRepo(selectedRepo.id);
      await refreshRepo(selectedRepo.id);
    });
  }

  async function onPush(setUpstream: boolean) {
    if (!selectedRepo) return;
    await run(async () => {
      await api.pushRepo(selectedRepo.id, setUpstream);
      await refreshRepo(selectedRepo.id);
    });
  }

  const remoteNames = useMemo(() => {
    const names = new Set<string>();
    for (const r of status?.remotes ?? []) names.add(r.name);
    for (const b of branches) {
      if (b.remote) names.add(b.remote);
    }
    return Array.from(names).sort();
  }, [status?.remotes, branches]);

  const remoteUrlByName = useMemo(() => {
    const map = new Map<string, string>();
    for (const r of status?.remotes ?? []) {
      map.set(r.name, r.url);
    }
    return map;
  }, [status?.remotes]);

  const scopeCounts = useMemo(() => {
    const counts: Record<string, number> = { local: 0 };
    for (const name of remoteNames) counts[name] = 0;
    for (const b of branches) {
      if (!b.isRemote) {
        counts.local += 1;
      } else if (b.remote) {
        counts[b.remote] = (counts[b.remote] ?? 0) + 1;
      }
    }
    return counts;
  }, [branches, remoteNames]);

  useEffect(() => {
    if (branchScope !== "local" && !remoteNames.includes(branchScope)) {
      setBranchScope("local");
    }
  }, [branchScope, remoteNames]);

  const visibleBranches = useMemo(() => {
    const q = filter.toLowerCase();
    return branches.filter((b) => {
      if (q && !b.name.toLowerCase().includes(q)) return false;
      if (branchScope === "local") return !b.isRemote;
      // remote tab: remote-tracking refs for this remote
      return b.isRemote && b.remote === branchScope;
    });
  }, [branches, filter, branchScope]);

  const activeRemoteUrl =
    branchScope !== "local" ? remoteUrlByName.get(branchScope) : undefined;

  return (
    <div className="app">
      <aside className="panel">
        <div className="panel-header">
          <h1 className="brand">GitSwitch</h1>
          <button className="btn btn-primary btn-sm" onClick={onAddRepo} disabled={busy}>
            添加
          </button>
        </div>
        <div className="panel-body">
          {!config?.repos.length && (
            <div className="empty">添加本地 Git 仓库开始管理</div>
          )}
          <div className="repo-list">
            {config?.repos.map((repo) => (
              <button
                key={repo.id}
                className={`repo-item ${selectedRepo?.id === repo.id ? "active" : ""}`}
                onClick={() => onSelectRepo(repo.id)}
              >
                <span className="name">{repo.name}</span>
                <span className="path">{repo.path}</span>
              </button>
            ))}
          </div>
          {selectedRepo && (
            <div style={{ marginTop: 12 }}>
              <button
                className="btn btn-danger btn-sm"
                onClick={() => onRemoveRepo(selectedRepo.id)}
                disabled={busy}
              >
                移除当前仓库
              </button>
            </div>
          )}
        </div>
      </aside>

      <main className="panel main-panel">
        <div className="panel-header">
          <h2>{selectedRepo ? selectedRepo.name : "仓库"}</h2>
          <div className="row">
            <button
              className="btn btn-sm"
              onClick={() => selectedRepo && refreshRepo(selectedRepo.id)}
              disabled={!selectedRepo || busy}
            >
              刷新
            </button>
            <button className="btn btn-sm" onClick={onFetch} disabled={!selectedRepo || busy}>
              Fetch
            </button>
            <button
              className="btn btn-sm"
              onClick={() => onPush(false)}
              disabled={!selectedRepo || busy}
            >
              Push
            </button>
            <button
              className="btn btn-sm"
              onClick={() => onPush(true)}
              disabled={!selectedRepo || busy}
            >
              Push -u
            </button>
          </div>
        </div>

        {error && <div className="error">{error}</div>}

        {status && (
          <div className="status-bar">
            <span className="pill ok">{status.currentBranch ?? "(detached)"}</span>
            {status.isDirty ? (
              <span className="pill warn">有未提交改动</span>
            ) : (
              <span className="pill">工作区干净</span>
            )}
            {(status.ahead > 0 || status.behind > 0) && (
              <span className="pill">
                ↑{status.ahead} ↓{status.behind}
              </span>
            )}
            <span className="meta">
              {status.userName || "?"} &lt;{status.userEmail || "?"}&gt;
            </span>
            {status.sshCommand && (
              <span className="meta" title={status.sshCommand}>
                SSH 已配置
              </span>
            )}
            {boundProfile && (
              <span className="pill ok">Profile: {boundProfile.name}</span>
            )}
          </div>
        )}

        <div className="tabs">
          <button
            className={`tab ${centerTab === "branches" ? "active" : ""}`}
            onClick={() => setCenterTab("branches")}
            disabled={!selectedRepo}
          >
            分支
            {selectedRepo ? ` (${branches.length})` : ""}
          </button>
          <button
            className={`tab ${centerTab === "commits" ? "active" : ""}`}
            onClick={() => setCenterTab("commits")}
            disabled={!selectedRepo}
          >
            提交
            {selectedRepo ? ` (${parsedLogs.length})` : ""}
          </button>
          <button
            className={`tab ${centerTab === "remotes" ? "active" : ""}`}
            onClick={() => setCenterTab("remotes")}
            disabled={!selectedRepo}
          >
            Remotes
            {selectedRepo && status?.remotes
              ? ` (${status.remotes.length})`
              : ""}
          </button>
        </div>

        {!selectedRepo ? (
          <div className="panel-body">
            <div className="empty">从左侧选择仓库</div>
          </div>
        ) : centerTab === "branches" ? (
          <>
            <div className="subtabs">
              <button
                className={`subtab ${branchScope === "local" ? "active" : ""}`}
                onClick={() => setBranchScope("local")}
              >
                本地 ({scopeCounts.local ?? 0})
              </button>
              {remoteNames.map((name) => (
                <button
                  key={name}
                  className={`subtab ${branchScope === name ? "active" : ""}`}
                  onClick={() => setBranchScope(name)}
                  title={remoteUrlByName.get(name) || name}
                >
                  {name} ({scopeCounts[name] ?? 0})
                </button>
              ))}
            </div>
            {activeRemoteUrl && (
              <div className="scope-banner">
                <span className="pill remote-pill">{branchScope}</span>
                <span className="meta truncate" title={activeRemoteUrl}>
                  {activeRemoteUrl}
                </span>
              </div>
            )}
            <div className="toolbar">
              <input
                className="toolbar-input grow"
                placeholder={
                  branchScope === "local"
                    ? "筛选本地分支…"
                    : `筛选 ${branchScope} 分支…`
                }
                value={filter}
                onChange={(e) => setFilter(e.target.value)}
              />
              {branchScope === "local" && (
                <>
                  <input
                    className="toolbar-input"
                    placeholder="新分支名"
                    value={newBranch}
                    onChange={(e) => setNewBranch(e.target.value)}
                    onKeyDown={(e) => e.key === "Enter" && onCreateBranch()}
                  />
                  <button
                    className="btn btn-primary btn-sm"
                    onClick={onCreateBranch}
                    disabled={busy || !newBranch.trim()}
                  >
                    创建并切换
                  </button>
                </>
              )}
            </div>
            <div className="panel-body">
              {!visibleBranches.length && (
                <div className="empty">
                  {branchScope === "local"
                    ? "没有匹配的本地分支"
                    : `没有匹配的 ${branchScope} 远程分支`}
                </div>
              )}
              <div className="branch-list">
                {visibleBranches.map((b) => (
                  <div
                    key={`${b.isRemote ? "r" : "l"}:${b.name}`}
                    className={`branch-item ${b.isCurrent ? "current" : ""}`}
                  >
                    <div className="branch-row">
                      <span className={`name ${b.isRemote ? "remote" : ""}`}>
                        {b.isCurrent ? "● " : "○ "}
                        {b.isRemote && b.remote
                          ? b.name.slice(b.remote.length + 1) || b.name
                          : b.name}
                      </span>
                      <div className="branch-actions">
                        {!b.isCurrent && (
                          <button
                            className="btn btn-sm"
                            disabled={busy}
                            onClick={() => onCheckout(b.name)}
                          >
                            切换
                          </button>
                        )}
                        {!b.isRemote && !b.isCurrent && (
                          <button
                            className="btn btn-sm btn-danger"
                            disabled={busy}
                            onClick={() => onDeleteBranch(b.name)}
                          >
                            删除
                          </button>
                        )}
                      </div>
                    </div>
                    <div className="branch-meta-row">
                      {b.isRemote ? (
                        <span className="meta">{b.name}</span>
                      ) : b.upstream ? (
                        <span className="meta">tracks → {b.upstream}</span>
                      ) : (
                        <span className="pill">无 upstream</span>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </>
        ) : centerTab === "commits" ? (
          <div className="panel-body">
            {!parsedLogs.length && <div className="empty">暂无提交记录</div>}
            <ul className="commit-list">
              {parsedLogs.map((c) => (
                <li key={c.raw} className="commit-item">
                  <div className="commit-top">
                    <span className="commit-hash">{c.hash}</span>
                    <span className="commit-when">{c.when}</span>
                  </div>
                  <div className="commit-subject">{c.subject}</div>
                  <div className="commit-author">{c.author}</div>
                </li>
              ))}
            </ul>
          </div>
        ) : (
          <div className="panel-body">
            {!status?.remotes?.length && (
              <div className="empty">没有配置 remote</div>
            )}
            <ul className="remote-list">
              {status?.remotes.map((r) => (
                <li key={`${r.name}-${r.url}`} className="remote-item">
                  <strong>{r.name}</strong>
                  <span className="mono">{r.url}</span>
                </li>
              ))}
            </ul>
          </div>
        )}
      </main>

      <aside className="panel">
        <div className="panel-header">
          <h2>Profiles</h2>
          <div className="row">
            <button
              className="btn btn-sm"
              onClick={onImportSshKeys}
              disabled={busy}
              title="扫描 ~/.ssh，把缺失的密钥导入为 Profile"
            >
              导入 SSH
            </button>
            <button
              className="btn btn-primary btn-sm"
              onClick={() => setEditing(emptyProfile())}
              disabled={busy}
            >
              新增
            </button>
          </div>
        </div>
        <div className="panel-body">
          {sshKeys.length > 0 && (
            <p className="meta" style={{ marginBottom: 10, lineHeight: 1.4 }}>
              本机发现 {sshKeys.length} 把密钥：
              {sshKeys.map((k) => k.name).join(", ")}
            </p>
          )}
          {!config?.profiles.length && (
            <div className="empty">还没有 Profile，点右上角新增或导入 SSH</div>
          )}
          <div className="profile-list">
            {config?.profiles.map((p) => (
              <div
                key={p.id}
                className="profile-item"
                style={{ border: "1px solid var(--border)" }}
              >
                <strong>{p.name || "(未命名)"}</strong>
                <span className="meta">
                  {p.userName || "?"} &lt;{p.userEmail || "?"}&gt;
                </span>
                <span className="meta">{p.sshKeyPath || "无 SSH key"}</span>
                <div className="row" style={{ marginTop: 8 }}>
                  <button
                    className="btn btn-primary btn-sm"
                    disabled={!selectedRepo || busy}
                    onClick={() => onApplyProfile(p.id)}
                  >
                    应用到当前仓
                  </button>
                  <button
                    className="btn btn-sm"
                    onClick={() => setEditing({ ...p })}
                  >
                    编辑
                  </button>
                  <button
                    className="btn btn-sm btn-danger"
                    onClick={() => onDeleteProfile(p.id)}
                  >
                    删
                  </button>
                </div>
              </div>
            ))}
          </div>
          <p className="meta" style={{ marginTop: 16, lineHeight: 1.5 }}>
            「应用到当前仓」写入该仓 local{" "}
            <span className="mono">user.*</span> 与{" "}
            <span className="mono">core.sshCommand</span>。
          </p>
        </div>
      </aside>

      {editing && (
        <div
          className="modal-backdrop"
          onClick={() => !busy && setEditing(null)}
          role="presentation"
        >
          <div
            className="modal"
            onClick={(e) => e.stopPropagation()}
            role="dialog"
            aria-modal="true"
            aria-labelledby="profile-modal-title"
          >
            <div className="modal-header">
              <h3 id="profile-modal-title">
                {editing.id ? "编辑 Profile" : "新增 Profile"}
              </h3>
              <button
                className="btn btn-sm"
                onClick={() => setEditing(null)}
                disabled={busy}
              >
                关闭
              </button>
            </div>
            <div className="modal-body">
              <div className="field">
                <label>名称</label>
                <input
                  value={editing.name}
                  onChange={(e) =>
                    setEditing({ ...editing, name: e.target.value })
                  }
                  autoFocus
                />
              </div>
              <div className="field">
                <label>user.name</label>
                <input
                  value={editing.userName}
                  onChange={(e) =>
                    setEditing({ ...editing, userName: e.target.value })
                  }
                />
              </div>
              <div className="field">
                <label>user.email</label>
                <input
                  value={editing.userEmail}
                  onChange={(e) =>
                    setEditing({ ...editing, userEmail: e.target.value })
                  }
                />
              </div>
              <div className="field">
                <label>SSH 私钥</label>
                <select
                  value={editing.sshKeyPath}
                  onChange={(e) => {
                    const path = e.target.value;
                    const key = sshKeys.find((k) => k.path === path);
                    setEditing({
                      ...editing,
                      sshKeyPath: path,
                      name:
                        editing.name ||
                        key?.comment ||
                        key?.name ||
                        editing.name,
                    });
                  }}
                >
                  <option value="">（不设置 SSH）</option>
                  {sshKeys.map((k) => (
                    <option key={k.path} value={k.path}>
                      {k.name}
                      {k.comment ? ` — ${k.comment}` : ""}
                    </option>
                  ))}
                  {editing.sshKeyPath &&
                    !sshKeys.some((k) => k.path === editing.sshKeyPath) && (
                      <option value={editing.sshKeyPath}>
                        {editing.sshKeyPath}（未找到文件）
                      </option>
                    )}
                </select>
              </div>
              <div className="field">
                <label>或手动填写路径</label>
                <input
                  placeholder="~/.ssh/id_ed25519_work"
                  value={editing.sshKeyPath}
                  onChange={(e) =>
                    setEditing({ ...editing, sshKeyPath: e.target.value })
                  }
                />
              </div>
              <div className="field">
                <label>SSH Host 别名（备注）</label>
                <input
                  placeholder="github-work"
                  value={editing.sshHostAlias}
                  onChange={(e) =>
                    setEditing({ ...editing, sshHostAlias: e.target.value })
                  }
                />
              </div>
            </div>
            <div className="modal-footer">
              <button
                className="btn"
                onClick={() => setEditing(null)}
                disabled={busy}
              >
                取消
              </button>
              <button
                className="btn btn-primary"
                onClick={onSaveProfile}
                disabled={busy || !editing.name.trim()}
              >
                保存
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

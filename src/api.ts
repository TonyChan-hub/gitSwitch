import { invoke } from "@tauri-apps/api/core";
import type {
  AppConfig,
  BranchInfo,
  Profile,
  RepoStatus,
  SshKeyInfo,
} from "./types";

export const api = {
  getConfig: () => invoke<AppConfig>("get_config"),
  addRepo: (path: string, name?: string) =>
    invoke<AppConfig>("add_repo", { path, name: name ?? null }),
  removeRepo: (repoId: string) => invoke<AppConfig>("remove_repo", { repoId }),
  selectRepo: (repoId: string) => invoke<AppConfig>("select_repo", { repoId }),
  upsertProfile: (profile: Profile) =>
    invoke<AppConfig>("upsert_profile", { profile }),
  deleteProfile: (profileId: string) =>
    invoke<AppConfig>("delete_profile", { profileId }),
  bindRepoProfile: (repoId: string, profileId: string | null) =>
    invoke<AppConfig>("bind_repo_profile", { repoId, profileId }),
  listBranches: (repoId: string) =>
    invoke<BranchInfo[]>("list_branches", { repoId }),
  checkoutBranch: (repoId: string, branch: string) =>
    invoke<void>("checkout_branch", { repoId, branch }),
  createBranch: (repoId: string, name: string, checkout: boolean) =>
    invoke<void>("create_branch", { repoId, name, checkout }),
  deleteBranch: (repoId: string, name: string, force: boolean) =>
    invoke<void>("delete_branch", { repoId, name, force }),
  getRepoStatus: (repoId: string) =>
    invoke<RepoStatus>("get_repo_status", { repoId }),
  applyProfile: (repoId: string, profileId: string) =>
    invoke<RepoStatus>("apply_profile", { repoId, profileId }),
  fetchRepo: (repoId: string) => invoke<string>("fetch_repo", { repoId }),
  pushRepo: (repoId: string, setUpstream: boolean) =>
    invoke<string>("push_repo", { repoId, setUpstream }),
  recentLog: (repoId: string, limit = 12) =>
    invoke<string[]>("recent_log", { repoId, limit }),
  listSshKeys: () => invoke<SshKeyInfo[]>("list_ssh_keys"),
  importMissingSshProfiles: () =>
    invoke<AppConfig>("import_missing_ssh_profiles"),
};

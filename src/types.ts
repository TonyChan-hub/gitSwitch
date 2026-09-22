export type Profile = {
  id: string;
  name: string;
  userName: string;
  userEmail: string;
  sshKeyPath: string;
  sshHostAlias: string;
};

export type Repo = {
  id: string;
  name: string;
  path: string;
  profileId?: string | null;
};

export type AppConfig = {
  repos: Repo[];
  profiles: Profile[];
  selectedRepoId?: string | null;
};

export type BranchInfo = {
  name: string;
  isCurrent: boolean;
  isRemote: boolean;
  upstream?: string | null;
  /** e.g. "origin" — tracked remote for local branches, or owning remote for remote refs */
  remote?: string | null;
};

export type RemoteInfo = {
  name: string;
  url: string;
};

export type SshKeyInfo = {
  name: string;
  path: string;
  pubPath?: string | null;
  comment?: string | null;
};

export type RepoStatus = {
  path: string;
  currentBranch?: string | null;
  isDirty: boolean;
  ahead: number;
  behind: number;
  remotes: RemoteInfo[];
  userName?: string | null;
  userEmail?: string | null;
  sshCommand?: string | null;
};



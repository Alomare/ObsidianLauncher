// The GitHub account (core/src/github.rs): who is signed in, the device sign-in, and the calls made as
// that player. One small store the Account tab, the nav icon and the Store pages all read.
import { useSyncExternalStore } from "react";
import { call, errorText } from "./api";
import { t } from "./i18n";
import { notify } from "./notices";
import { openUrl } from "./opener";

export type Account = { login: string; avatar: string | null; scopes: string[]; syncGist: string | null };
export type DeviceCode = { deviceCode: string; userCode: string; verificationUri: string; interval: number; expiresIn: number };
type Poll = { state: "pending"; interval: number } | { state: "done"; account: Account } | { state: "denied" } | { state: "expired" };
export type RepoInfo = {
  fullName: string; private: boolean; fork: boolean; archived: boolean; hasIssues: boolean; stars: number; owner: string;
  ownerIsOrg: boolean; push: boolean; defaultBranch: string; openIssues: number;
};
export type Templates = { files: { name: string; text: string }[]; blank: boolean; hasIssues: boolean };
export type Report = {
  repo: string; number: number; title: string; modId: string; modName: string; url: string; created: string;
  state: "open" | "closed" | "fixed" | "deleted" | string; comments: number; seen: number; updated: string;
};
export type Message = { author: string; avatar: string | null; association: string; body: string; created: string };
export type OpenIssue = { number: number; title: string; author: string; comments: number; created: string; url: string; labels: string[] };
export type SyncState = { url: string | null; updated: string | null };

/** `gist` for shared profiles and sync, `repo` for early access. */
export type Extra = "gist" | "repo";

type State = {
  account: Account | null;
  /** A sign-in in progress: the code to type, and what it asks for. */
  signing: (DeviceCode & { extra: Extra[] }) | null;
  /** The reports filed from this PC, as last read. */
  reports: Report[];
};

let state: State = { account: null, signing: null, reports: [] };
const listeners = new Set<() => void>();
const set = (patch: Partial<State>) => {
  state = { ...state, ...patch };
  listeners.forEach((f) => f());
};
export const useGitHub = () =>
  useSyncExternalStore((f) => { listeners.add(f); return () => { listeners.delete(f); }; }, () => state);
export const gitHubState = () => state;

export const hasScope = (a: Account | null, scope: string) => !!a && (a.scopes.includes(scope) || (scope === "public_repo" && a.scopes.includes("repo")));
export const unread = (r: Report) => Math.max(0, r.comments - r.seen);

/** At startup: the saved account at once, then checked with GitHub, then the reports and their replies. */
export async function startGitHub() {
  try {
    set({ account: await call<Account | null>("github_account"), reports: await call<Report[]>("github_reports") });
  } catch { /* no account file */ }
  if (!state.account) return;
  try {
    const account = await call<Account | null>("github_verify");
    set({ account });
    if (!account) notify("info", t("app.github.signedOutByGitHub"));
  } catch { /* offline: keep the saved account */ }
  if (state.account) refreshReports(true);
}

/** Asks GitHub where the reports stand; `announce` tells of new replies with a notice. */
export async function refreshReports(announce = false) {
  if (!state.reports.length) return;
  try {
    const before = new Map(state.reports.map((r) => [`${r.repo}#${r.number}`, r.comments]));
    const reports = await call<Report[]>("github_reports_refresh");
    set({ reports });
    if (announce) {
      const fresh = reports.filter((r) => unread(r) > 0 && r.comments > (before.get(`${r.repo}#${r.number}`) ?? 0));
      const any = fresh.length ? fresh : reports.filter((r) => unread(r) > 0);
      if (any.length === 1) notify("info", t("app.github.newReply", any[0].modName || any[0].repo, any[0].title));
      else if (any.length > 1) notify("info", t("app.github.newReplies", any.length));
    }
  } catch { /* offline */ }
}

export const setReports = (reports: Report[]) => set({ reports });

let polling = 0;

/** Starts a sign-in (or one for more access: `extra`), opens github.com/login/device and polls until
 *  the player has entered the code, refused, or the code ran out. */
export async function signIn(extra: Extra[] = []): Promise<boolean> {
  const run = ++polling;
  let code: DeviceCode;
  try {
    code = await call<DeviceCode>("github_sign_in_start", { extra });
  } catch (e) {
    notify("error", errorText(e));
    return false;
  }
  set({ signing: { ...code, extra } });
  openUrl(code.verificationUri).catch(() => {});
  let interval = code.interval;
  const until = Date.now() + code.expiresIn * 1000;
  while (run === polling && Date.now() < until) {
    await new Promise((r) => setTimeout(r, interval * 1000));
    if (run !== polling) return false;
    let p: Poll;
    try {
      p = await call<Poll>("github_sign_in_poll", { deviceCode: code.deviceCode, interval });
    } catch (e) {
      set({ signing: null });
      notify("error", errorText(e));
      return false;
    }
    if (p.state === "pending") { interval = p.interval; continue; }
    set({ signing: null });
    if (p.state === "done") {
      set({ account: p.account });
      notify("info", t("app.github.signedIn", p.account.login));
      return true;
    }
    notify("info", p.state === "denied" ? t("app.github.denied") : t("app.github.expired"));
    return false;
  }
  if (run === polling) {
    set({ signing: null });
    notify("info", t("app.github.expired"));
  }
  return false;
}

export function cancelSignIn() {
  polling++;
  set({ signing: null });
}

export async function signOut() {
  try {
    await call("github_sign_out");
    set({ account: null });
  } catch (e) {
    notify("error", errorText(e));
  }
}

/** Makes sure the account has `scope`, asking for it with a new sign-in when it doesn't. */
export async function ensureScope(scope: Extra): Promise<boolean> {
  if (hasScope(state.account, scope)) return true;
  return signIn([scope]);
}

/** Copies the code to type at github.com. */
export const copyCode = (code: string) => navigator.clipboard.writeText(code).then(() => notify("info", t("app.github.codeCopied")), () => {});

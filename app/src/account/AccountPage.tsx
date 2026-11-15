// The Account tab: the GitHub sign-in and the access it holds, the reports filed from this PC with
// their replies, and sync of profiles and appearance between PCs.
// Built from the Settings parts: groups under headings, a status card per group with the buttons under it,
// the reports as a table like the Other patches table, and a report as boxes like a Store mod page.
import { useEffect, useMemo, useState, type ReactNode } from "react";
import { call, errorText, setSnapshot, taskText } from "../api";
import { withTask } from "../busy";
import { Confirm } from "../Confirm";
import {
  cancelSignIn, copyCode, ensureScope, hasScope, refreshReports, setReports, signIn, signOut, unread, useGitHub,
  type Message, type Report, type SyncState,
} from "../github";
import { CheckIcon, ChevronIcon, CopyIcon, ExportIcon, GitHubIcon, ImportIcon, OutboundIcon } from "../Icons";
import { language, t } from "../i18n";
import { Markdown, parse } from "../Markdown";
import { notify } from "../notices";
import { openUrl } from "../opener";
import { Group } from "../SettingRow";
import { applySettings } from "../settings";
import type { AppSettings, Snapshot } from "../types";
import "./account.css";
import { PadGlyph } from "../pad/PadGlyph";

export type AccountTab = "github" | "reports" | "sync";
const TABS: AccountTab[] = ["github", "reports", "sync"];

const day = (iso: string | null) => (iso ? new Date(iso).toLocaleDateString(language(), { dateStyle: "medium" }) : "");
const when = (iso: string | null) => (iso ? new Date(iso).toLocaleString(language(), { dateStyle: "medium", timeStyle: "short" }) : "");
const open = (url: string) => openUrl(url).catch((e) => notify("error", errorText(e)));

export function AccountPage({ tab, onTab }: { tab: AccountTab; onTab: (t: AccountTab) => void }) {
  return (
    <div className="settings account">
      <div className="head">
        <h1 className="cap">{t("app.account.title")}</h1>
        <div className="subtabs" role="tablist">
          {TABS.map((id) => (
            <button key={id} role="tab" aria-selected={tab === id} className={tab === id ? "cap cur" : "cap"} onClick={() => onTab(id)}>
              {t(`app.account.${id}`)}
            </button>
          ))}
        </div>
      </div>
      <div className="view settings-body" key={tab}>
        {tab === "github" ? <GitHubTab /> : tab === "reports" ? <ReportsTab /> : <SyncTab />}
      </div>
    </div>
  );
}

/** GitHub serves avatars at any size through `s`; ask for twice the shown size, for sharp edges. */
function sized(url: string, px: number) {
  try {
    const u = new URL(url);
    if (u.hostname === "avatars.githubusercontent.com") u.searchParams.set("s", String(px * 2));
    return u.toString();
  } catch {
    return url;
  }
}

/** The GitHub picture in a circle, or the first letter of the login when there is none or it fails to load. */
function Avatar({ url, login, small }: { url: string | null; login: string; small?: boolean }) {
  const [failed, setFailed] = useState(false);
  const cls = small ? "acc-avatar small" : "acc-avatar";
  if (!url || failed) return <span className={`${cls} letter`} aria-hidden="true">{[...login.trim()][0]?.toUpperCase() ?? "?"}</span>;
  return <img className={cls} src={sized(url, small ? 24 : 48)} alt="" draggable={false} onError={() => setFailed(true)} />;
}

/** The status card of a group (the shape of the Game card in Settings): a lead at the left, a title, a line under it. */
function Card({ tone, lead, title, children }: { tone?: "ok" | "busy"; lead: ReactNode; title: ReactNode; children?: ReactNode }) {
  return (
    <div className={tone ? `game-card acc-card ${tone}` : "game-card acc-card"}>
      {lead}
      <div className="game-text">
        <b>{title}</b>
        {children}
      </div>
    </div>
  );
}

/** The code to type at github.com/login/device while a sign-in waits. */
function Signing() {
  const { signing } = useGitHub();
  if (!signing) return null;
  return (
    <>
      <Card tone="busy" lead={<span className="acc-avatar acc-spin" role="status" aria-label={t("app.account.waiting")}><span className="upd-spin" /></span>}
        title={<output className="acc-code" aria-label={t("app.account.code")}>{signing.userCode}</output>}>
        <small>{t("app.account.codeLead")}</small>
      </Card>
      <div className="set-btns">
        <button className="btn small" onClick={() => copyCode(signing.userCode)}><CopyIcon />{t("app.account.copy")}</button>
        <button className="btn small" onClick={() => open(signing.verificationUri)}><OutboundIcon />{t("app.account.openPage")}</button>
        <button data-pad-back className="btn small" onClick={cancelSignIn}><PadGlyph role="east" />{t("app.cancel")}</button>
      </div>
    </>
  );
}

function GitHubTab() {
  const { account, signing } = useGitHub();
  const [leaving, setLeaving] = useState(false);
  const revoke = () => call<string>("github_revoke_url").then(open, (e) => notify("error", errorText(e)));
  const rows: { scope: string; extra?: "gist" | "repo"; label: string; note: string }[] = [
    { scope: "public_repo", label: t("app.account.scopePublic"), note: t("app.account.scopePublicNote") },
    { scope: "gist", extra: "gist", label: t("app.account.scopeGist"), note: t("app.account.scopeGistNote") },
    { scope: "repo", extra: "repo", label: t("app.account.scopeRepo"), note: t("app.account.scopeRepoNote") },
  ];
  return (
    <div className="set-page">
      <section className="set-sec">
        {signing ? <Signing /> : account ? (
          <>
            <Card tone="ok" lead={<Avatar key={account.avatar} url={account.avatar} login={account.login} />}
              title={<button className="link acc-login" onClick={() => open(`https://github.com/${account.login}`)}>{account.login}</button>}>
              <small>{t("app.account.revokeNote")}</small>
            </Card>
            <div className="set-btns">
              <button className="btn small" onClick={() => setLeaving(true)}>{t("app.account.signOut")}</button>
              <button className="btn small" onClick={revoke}><OutboundIcon />{t("app.account.revoke")}</button>
            </div>
          </>
        ) : (
          <>
            <Card lead={<span className="acc-avatar letter" aria-hidden="true"><GitHubIcon /></span>} title={t("app.account.signedOut")}>
              <small>{t("app.account.signedOutLead")}</small>
            </Card>
            <div className="set-btns">
              <button className="btn small play" onClick={() => signIn()}><GitHubIcon />{t("app.account.signIn")}</button>
            </div>
          </>
        )}
      </section>
      <Group title={t("app.account.access")}>
        <div className="patch-table acc-access" role="table" aria-label={t("app.account.access")}>
          {rows.map((r) => {
            const granted = hasScope(account, r.scope);
            return (
              <div key={r.scope} className="pt-row" role="row">
                <span className="acc-access-text" role="cell">
                  <b>{r.label}</b>
                  <small>{r.note}</small>
                </span>
                <span className="acc-access-ctl" role="cell">
                  {granted
                    ? <span className="acc-granted"><CheckIcon />{t("app.account.granted")}</span>
                    : <button className="btn small" disabled={!account || !!signing} onClick={() => ensureScope(r.extra!)}>{t("app.account.allow")}</button>}
                </span>
              </div>
            );
          })}
        </div>
      </Group>
      {leaving && (
        <Confirm title={t("app.account.signOutTitle")} text={t("app.account.signOutText")} confirm={t("app.account.signOut")}
          onCancel={() => setLeaving(false)} onConfirm={() => { setLeaving(false); signOut(); }} />
      )}
    </div>
  );
}

const STATES = ["open", "closed", "fixed", "deleted"];
function StateBadge({ state }: { state: string }) {
  const s = STATES.includes(state) ? state : "open";
  return <span className={`acc-state ${s}`}>{t(`app.account.state.${s}`)}</span>;
}

function ReportsTab() {
  const { reports } = useGitHub();
  const [current, setCurrent] = useState<Report | null>(null);
  useEffect(() => { refreshReports(); }, []);
  if (current) {
    const fresh = reports.find((r) => r.repo === current.repo && r.number === current.number) ?? current;
    return <Thread report={fresh} onBack={() => setCurrent(null)} />;
  }
  return (
    <div className="set-page">
      <section className="set-sec">
        {!reports.length ? <p className="set-note">{t("app.account.reportsEmpty")}</p> : (
          <div className="patch-table acc-reports" role="table" aria-label={t("app.account.reports")}>
            <div className="pt-row pt-head" role="row">
              <span role="columnheader">{t("app.account.col.report")}</span>
              <span role="columnheader">{t("app.account.col.mod")}</span>
              <span role="columnheader">{t("app.account.col.state")}</span>
              <span role="columnheader">{t("app.account.col.opened")}</span>
            </div>
            {reports.map((r) => {
              const n = unread(r);
              const tip = n === 1 ? t("app.account.unreadOne") : t("app.account.unread", n);
              return (
                <button key={`${r.repo}#${r.number}`} className={n ? "pt-row acc-report new" : "pt-row acc-report"} role="row" onClick={() => setCurrent(r)}>
                  <span className="acc-report-title" role="cell">
                    <b>{r.title}</b>
                    {n > 0 && <span className="acc-unread" aria-label={tip} data-tip={tip}>{n}</span>}
                  </span>
                  <span className="acc-report-mod" role="cell">{r.modName || r.repo}</span>
                  <span role="cell"><StateBadge state={r.state} /></span>
                  <span role="cell">{day(r.created)}</span>
                </button>
              );
            })}
          </div>
        )}
      </section>
    </div>
  );
}

const AUTHORS = ["OWNER", "MEMBER", "COLLABORATOR"];

function Thread({ report, onBack }: { report: Report; onBack: () => void }) {
  const { account, signing } = useGitHub();
  const [messages, setMessages] = useState<Message[] | null>(null);
  const [text, setText] = useState("");
  const [sending, setSending] = useState(false);
  useEffect(() => {
    let live = true;
    call<Message[]>("github_thread", { repo: report.repo, number: report.number })
      .then(async (m) => {
        if (!live) return;
        setMessages(m);
        setReports(await call<Report[]>("github_reports"));
      })
      .catch((e) => { if (live) { setMessages([]); notify("error", errorText(e)); } });
    return () => { live = false; };
  }, [report.repo, report.number]);
  const send = async () => {
    if (!text.trim()) return;
    setSending(true);
    try {
      setMessages(await withTask(t("app.account.sending"), () => call<Message[]>("github_reply", { repo: report.repo, number: report.number, body: text })));
      setText("");
      setReports(await call<Report[]>("github_reports"));
    } catch (e) {
      notify("error", errorText(e));
    } finally {
      setSending(false);
    }
  };
  const forget = async () => {
    try {
      setReports(await call<Report[]>("github_report_forget", { repo: report.repo, number: report.number }));
      onBack();
    } catch (e) {
      notify("error", errorText(e));
    }
  };
  return (
    <div className="acc-thread">
      <div className="acc-thread-head">
        <button data-pad-back className="btn small acc-back" onClick={onBack}><ChevronIcon /><PadGlyph role="east" swap />{t("app.account.back")}</button>
        <div className="acc-thread-title">
          <h2>{report.title}</h2>
          <StateBadge state={report.state} />
        </div>
        <span className="acc-meta">{[report.modName || report.repo, day(report.created)].filter(Boolean).join(" · ")}</span>
      </div>
      {!messages && <p className="acc-wait"><span className="upd-spin" aria-hidden="true" />{t("app.account.loading")}</p>}
      {messages && (
        <ol className="acc-messages">
          {messages.map((m, i) => <MessageRow key={i} m={m} />)}
        </ol>
      )}
      <section className="store-box acc-reply">
        {signing ? <Signing /> : account ? (
          <textarea value={text} onChange={(e) => setText(e.target.value)} placeholder={t("app.account.replyPlaceholder")} aria-label={t("app.account.replyPlaceholder")} rows={4} />
        ) : (
          <p className="set-note">{t("app.account.replySignIn")}</p>
        )}
        {!signing && (
          <div className="set-btns">
            {account
              ? <button className="btn small play" onClick={send} disabled={sending || !text.trim()}>{t("app.account.send")}</button>
              : <button className="btn small play" onClick={() => signIn()}><GitHubIcon />{t("app.account.signIn")}</button>}
            <button className="btn small" onClick={() => open(report.url)}><OutboundIcon />{t("app.account.openOnGitHub")}</button>
            <button className="btn small" onClick={forget}>{t("app.account.forget")}</button>
          </div>
        )}
      </section>
    </div>
  );
}

function MessageRow({ m }: { m: Message }) {
  const doc = useMemo(() => parse(m.body), [m.body]);
  return (
    <li className="store-box acc-message">
      <div className="acc-message-head">
        <Avatar key={m.avatar} url={m.avatar} login={m.author} small />
        <b>{m.author}</b>
        {AUTHORS.includes(m.association) && <span className="acc-author">{t("app.account.author")}</span>}
        <span className="acc-meta">{when(m.created)}</span>
      </div>
      <div className="acc-message-body"><Markdown tokens={doc.tokens} /></div>
    </li>
  );
}

type SyncDownloaded = { snapshot: Snapshot; downloaded: { profiles: string[]; installed: string[]; missing: string[]; updated: string }; settings: AppSettings };

function SyncTab() {
  const { account, signing } = useGitHub();
  const gist = hasScope(account, "gist");
  const [sync, setSync] = useState<SyncState | null>(null);
  const [asking, setAsking] = useState(false);
  useEffect(() => {
    if (!gist) return;
    let live = true;
    call<SyncState>("sync_state").then((s) => live && setSync(s), (e) => notify("error", errorText(e)));
    return () => { live = false; };
  }, [gist]);
  const upload = async () => {
    try {
      setSync(await withTask(taskText("sync_upload"), () => call<SyncState>("sync_upload")));
      notify("info", t("app.account.uploaded"));
    } catch (e) {
      notify("error", errorText(e));
    }
  };
  const download = async () => {
    setAsking(false);
    try {
      const r = await withTask(taskText("sync_download"), () => call<SyncDownloaded>("sync_download"), true);
      setSnapshot(r.snapshot);
      applySettings(r.settings);
      setSync((s) => ({ url: s?.url ?? null, updated: r.downloaded.updated || s?.updated || null }));
      notify("info", t("app.account.downloaded", r.downloaded.profiles.length, r.downloaded.installed.length));
      if (r.downloaded.missing.length) notify("error", t("app.account.missing", r.downloaded.missing.join(", ")));
    } catch (e) {
      notify("error", errorText(e));
    }
  };
  const dot = <span className="game-dot" aria-hidden="true" />;
  return (
    <div className="set-page">
      <section className="set-sec">
        {signing ? <Signing /> : !gist ? (
          <>
            <Card lead={dot} title={t("app.account.syncOff")}><small>{t("app.account.syncNeedsGist")}</small></Card>
            <div className="set-btns">
              <button className="btn small play" onClick={() => ensureScope("gist")}>{account ? t("app.account.allow") : t("app.account.signIn")}</button>
            </div>
          </>
        ) : (
          <>
            <Card tone={sync?.updated ? "ok" : undefined} lead={dot}
              title={sync?.updated ? t("app.account.syncLast", when(sync.updated)) : sync ? t("app.account.syncNone") : t("app.account.loading")}>
              <small>{t("app.account.syncLead")}</small>
            </Card>
            <div className="set-btns">
              <button className="btn small" onClick={upload}><ExportIcon />{t("app.account.upload")}</button>
              <button className="btn small" onClick={() => setAsking(true)} disabled={!sync?.url}><ImportIcon />{t("app.account.download")}</button>
              {sync?.url && <button className="btn small" onClick={() => open(sync.url!)}><OutboundIcon />{t("app.account.syncOpen")}</button>}
            </div>
          </>
        )}
      </section>
      {asking && (
        <Confirm title={t("app.account.downloadTitle")} text={t("app.account.downloadText")} confirm={t("app.account.download")}
          onCancel={() => setAsking(false)} onConfirm={download} />
      )}
    </div>
  );
}

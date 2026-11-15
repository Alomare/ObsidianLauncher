// The Account tab: the GitHub sign-in and the access it holds, the reports filed from this PC with
// their replies, and sync of profiles and appearance between PCs (plus shared profile links).
import { useEffect, useMemo, useState } from "react";
import { call, errorText, setSnapshot, taskText } from "../api";
import { withTask } from "../busy";
import { Confirm } from "../Confirm";
import {
  cancelSignIn, copyCode, ensureScope, hasScope, refreshReports, setReports, signIn, signOut, unread, useGitHub,
  type Message, type Report, type SyncState,
} from "../github";
import { CheckIcon, CopyIcon, GitHubIcon, OutboundIcon } from "../Icons";
import { language, t } from "../i18n";
import { Markdown, parse } from "../Markdown";
import { notify } from "../notices";
import { openUrl } from "../opener";
import { Group } from "../SettingRow";
import { applySettings } from "../settings";
import type { AppSettings, Snapshot } from "../types";
import "./account.css";

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

/** The code to type at github.com/login/device while a sign-in waits. */
function Signing() {
  const { signing } = useGitHub();
  if (!signing) return null;
  return (
    <section className="set-sec acc-signing">
      <p className="set-note">{t("app.account.codeLead")}</p>
      <output className="acc-code" aria-label={t("app.account.code")}>{signing.userCode}</output>
      <div className="set-btns">
        <button className="btn small" onClick={() => copyCode(signing.userCode)}><CopyIcon />{t("app.account.copy")}</button>
        <button className="btn small" onClick={() => open(signing.verificationUri)}><OutboundIcon />{t("app.account.openPage")}</button>
        <button className="btn small" onClick={cancelSignIn}>{t("app.cancel")}</button>
      </div>
      <p className="acc-wait"><span className="upd-spin" aria-hidden="true" />{t("app.account.waiting")}</p>
    </section>
  );
}

function GitHubTab() {
  const { account, signing } = useGitHub();
  const [leaving, setLeaving] = useState(false);
  if (signing) return <div className="set-page"><Signing /></div>;
  if (!account) {
    return (
      <div className="set-page">
        <section className="set-sec acc-out">
          <p className="set-note">{t("app.account.signedOutLead")}</p>
          <div className="set-btns">
            <button className="btn play acc-signin" onClick={() => signIn()}><GitHubIcon />{t("app.account.signIn")}</button>
          </div>
        </section>
      </div>
    );
  }
  const rows: { scope: string; extra?: "gist" | "repo"; label: string; note?: string }[] = [
    { scope: "public_repo", label: t("app.account.scopePublic") },
    { scope: "gist", extra: "gist", label: t("app.account.scopeGist") },
    { scope: "repo", extra: "repo", label: t("app.account.scopeRepo"), note: t("app.account.scopeRepoNote") },
  ];
  const revoke = () => call<string>("github_revoke_url").then(open, (e) => notify("error", errorText(e)));
  return (
    <div className="set-page">
      <section className="set-sec acc-who">
        {account.avatar ? <img className="acc-avatar" src={account.avatar} alt="" /> : <span className="acc-avatar" aria-hidden="true" />}
        <button className="link acc-login" onClick={() => open(`https://github.com/${account.login}`)}>{account.login}</button>
      </section>
      <Group title={t("app.account.access")}>
        <ul className="acc-access">
          {rows.map((r) => {
            const granted = hasScope(account, r.scope);
            return (
              <li key={r.scope}>
                <span className="acc-access-text">
                  <b>{r.label}</b>
                  {r.note && <small>{r.note}</small>}
                </span>
                {granted
                  ? <span className="acc-granted" role="img" aria-label={t("app.account.granted")} data-tip={t("app.account.granted")}><CheckIcon /></span>
                  : <button className="btn small" onClick={() => ensureScope(r.extra!)}>{t("app.account.allow")}</button>}
              </li>
            );
          })}
        </ul>
      </Group>
      <section className="set-sec">
        <div className="set-btns">
          <button className="btn small" onClick={() => setLeaving(true)}>{t("app.account.signOut")}</button>
          <button className="btn small" onClick={revoke}><OutboundIcon />{t("app.account.revoke")}</button>
        </div>
        <p className="set-note">{t("app.account.revokeNote")}</p>
      </section>
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
  if (!reports.length) return <div className="set-page"><p className="set-note acc-empty">{t("app.account.reportsEmpty")}</p></div>;
  return (
    <ul className="acc-reports">
      {reports.map((r) => {
        const n = unread(r);
        return (
          <li key={`${r.repo}#${r.number}`}>
            <button className={n ? "acc-report new" : "acc-report"} onClick={() => setCurrent(r)}>
              <span className="acc-report-text">
                <small>{r.modName || r.repo}</small>
                <b>{r.title}</b>
              </span>
              {n > 0 && <span className="acc-unread" aria-label={n === 1 ? t("app.account.unreadOne") : t("app.account.unread", n)}
                data-tip={n === 1 ? t("app.account.unreadOne") : t("app.account.unread", n)}>{n}</span>}
              <StateBadge state={r.state} />
              <span className="acc-date">{day(r.created)}</span>
            </button>
          </li>
        );
      })}
    </ul>
  );
}

const AUTHORS = ["OWNER", "MEMBER", "COLLABORATOR"];

function Thread({ report, onBack }: { report: Report; onBack: () => void }) {
  const { account } = useGitHub();
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
        <button className="link small acc-back" onClick={onBack}>{t("app.account.back")}</button>
        <h2>{report.title}</h2>
        <StateBadge state={report.state} />
        <span className="acc-date">{report.modName || report.repo}</span>
      </div>
      {!messages && <p className="acc-wait"><span className="upd-spin" aria-hidden="true" />{t("app.account.loading")}</p>}
      {messages && (
        <ol className="acc-messages">
          {messages.map((m, i) => <MessageRow key={i} m={m} />)}
        </ol>
      )}
      {account ? (
        <div className="acc-reply">
          <textarea value={text} onChange={(e) => setText(e.target.value)} placeholder={t("app.account.replyPlaceholder")} aria-label={t("app.account.replyPlaceholder")} rows={4} />
          <div className="set-btns">
            <button className="btn small play" onClick={send} disabled={sending || !text.trim()}>{t("app.account.send")}</button>
            <button className="btn small" onClick={() => open(report.url)}><OutboundIcon />{t("app.account.openOnGitHub")}</button>
            <button className="btn small" onClick={forget}>{t("app.account.forget")}</button>
          </div>
        </div>
      ) : (
        <div className="acc-reply">
          <p className="set-note">{t("app.account.replySignIn")}</p>
          <div className="set-btns">
            <button className="btn small play" onClick={() => signIn()}>{t("app.account.signIn")}</button>
            <button className="btn small" onClick={() => open(report.url)}><OutboundIcon />{t("app.account.openOnGitHub")}</button>
            <button className="btn small" onClick={forget}>{t("app.account.forget")}</button>
          </div>
        </div>
      )}
      <Signing />
    </div>
  );
}

function MessageRow({ m }: { m: Message }) {
  const doc = useMemo(() => parse(m.body), [m.body]);
  return (
    <li className="acc-message">
      <div className="acc-message-head">
        {m.avatar ? <img className="acc-avatar small" src={m.avatar} alt="" /> : <span className="acc-avatar small" aria-hidden="true" />}
        <b>{m.author}</b>
        {AUTHORS.includes(m.association) && <span className="acc-author">{t("app.account.author")}</span>}
        <span className="acc-date">{when(m.created)}</span>
      </div>
      <div className="acc-message-body"><Markdown tokens={doc.tokens} /></div>
    </li>
  );
}

type SyncDownloaded = { snapshot: Snapshot; downloaded: { profiles: string[]; installed: string[]; missing: string[]; updated: string }; settings: AppSettings };
type SharedImported = { snapshot: Snapshot; imported: { name: string; installed: string[]; kept: string[]; missing: string[]; guardrails: string[] } };

function SyncTab() {
  const { account, signing } = useGitHub();
  const gist = hasScope(account, "gist");
  const [sync, setSync] = useState<SyncState | null>(null);
  const [asking, setAsking] = useState(false);
  const [link, setLink] = useState("");
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
  const importLink = async () => {
    const value = link.trim();
    if (!value) return;
    try {
      const r = await withTask(taskText("import_shared"), () => call<SharedImported>("import_shared", { link: value }), true);
      setSnapshot(r.snapshot);
      setLink("");
      const n = r.imported.installed.length;
      notify("info", n === 1 ? t("app.profiles.importedOne", r.imported.name) : t("app.profiles.imported", r.imported.name, n));
      if (r.imported.missing.length) notify("error", t("app.account.missing", r.imported.missing.join(", ")));
      // The guardrails come along with a profile, as with a ZIP.
      const guards = r.imported.guardrails.map((g) => t(g === "freeze_progress" ? "app.guardrails.freeze" : "app.guardrails.achievements"));
      if (guards.length > 0) notify("info", t("app.profiles.importedGuardrails", r.imported.name, guards.join(", ")));
    } catch (e) {
      notify("error", errorText(e));
    }
  };
  return (
    <div className="set-page">
      {signing ? <Signing /> : (
        <Group title={t("app.account.syncTitle")}>
          {!gist ? (
            <>
              <p className="set-note">{t("app.account.syncNeedsGist")}</p>
              <div className="set-btns">
                <button className="btn small play" onClick={() => ensureScope("gist")}>{account ? t("app.account.allow") : t("app.account.signIn")}</button>
              </div>
            </>
          ) : (
            <>
              <p className="set-note acc-sync-state">
                {sync?.updated ? t("app.account.syncLast", when(sync.updated)) : sync ? t("app.account.syncNone") : t("app.account.loading")}
                {sync?.url && <button className="link small acc-inline" onClick={() => open(sync.url!)}>{t("app.account.syncOpen")}</button>}
              </p>
              <div className="set-btns">
                <button className="btn small" onClick={upload}>{t("app.account.upload")}</button>
                <button className="btn small" onClick={() => setAsking(true)} disabled={!sync?.url}>{t("app.account.download")}</button>
              </div>
            </>
          )}
        </Group>
      )}
      <Group title={t("app.account.importTitle")}>
        <form className="acc-import" onSubmit={(e) => { e.preventDefault(); importLink(); }}>
          <input value={link} onChange={(e) => setLink(e.target.value)} placeholder={t("app.account.importPlaceholder")} aria-label={t("app.account.importPlaceholder")} spellCheck={false} />
          <button className="btn small" type="submit" disabled={!link.trim()}>{t("app.account.import")}</button>
        </form>
        <p className="set-note">{t("app.account.importNote")}</p>
      </Group>
      {asking && (
        <Confirm title={t("app.account.downloadTitle")} text={t("app.account.downloadText")} confirm={t("app.account.download")}
          onCancel={() => setAsking(false)} onConfirm={download} />
      )}
    </div>
  );
}

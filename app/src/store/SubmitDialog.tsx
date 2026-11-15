// Submit a mod: the player names a repository of their own, Obsidian checks it (core/src/submit.rs)
// and, when nothing is wrong, opens a pull request to the Store index. The index runs the same
// checks on the pull request, and the maintainer merges it.
import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { call, errorText } from "../api";
import { withTask } from "../busy";
import { signIn, useGitHub } from "../github";
import { CheckIcon, GitHubIcon, WarnIcon } from "../Icons";
import { t } from "../i18n";
import { notify } from "../notices";
import { openUrl } from "../opener";
import "./github.css";
import { PadGlyph } from "../pad/PadGlyph";

type Problem = { code: string; detail: string };
type Checked = { repo: string; problems: Problem[]; preview: { id: string; name: string; summary: string; version: string; scope: string; authors: string[] } | null };

/** A problem in words: the catalog line, with the detail where it names something. */
function problemText(p: Problem) {
  const key = `app.submit.problem.${p.code}`;
  const why = p.code.endsWith("Text") ? t(`app.submit.why.${p.detail}`) : p.detail;
  const text = t(key, why);
  return text === key ? `${p.code}: ${p.detail}` : text;
}

export function SubmitDialog({ onClose }: { onClose: () => void }) {
  const { account, signing } = useGitHub();
  const [repo, setRepo] = useState("");
  const [checked, setChecked] = useState<Checked | null>(null);
  const [checking, setChecking] = useState(false);
  const [note, setNote] = useState("");
  const [sending, setSending] = useState(false);
  const [done, setDone] = useState<string | null>(null);
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => {
    input.current?.focus();
    const key = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [onClose]);

  const check = async () => {
    if (!repo.trim() || checking) return;
    setChecking(true);
    setChecked(null);
    try {
      setChecked(await withTask(t("app.submit.checking"), () => call<Checked>("submit_check", { repo: repo.trim() })));
    } catch (e) {
      notify("error", errorText(e));
    }
    setChecking(false);
  };
  const send = async () => {
    if (!checked || checked.problems.length || sending) return;
    setSending(true);
    try {
      setDone(await withTask(t("app.submit.sending"), () => call<string>("submit_send", { repo: checked.repo, note })));
    } catch (e) {
      notify("error", errorText(e));
    }
    setSending(false);
  };

  let content;
  if (!account) {
    content = (
      <>
        <p>{t("app.submit.signInText")}</p>
        {signing && <p className="rf-code">{t("app.report.typeCode")} <b className="mono">{signing.userCode}</b></p>}
        <div className="modal-foot">
          <button data-pad-back className="btn" onClick={onClose}><PadGlyph role="east" />{t("app.cancel")}</button>
          <button className="btn play" onClick={() => signIn()} disabled={!!signing}><GitHubIcon />{signing ? t("app.report.signingIn") : t("app.report.signIn")}</button>
        </div>
      </>
    );
  } else if (done) {
    content = (
      <>
        <p>{t("app.submit.done")}</p>
        <div className="modal-foot">
          <button className="btn" onClick={() => openUrl(done).catch(() => {})}><GitHubIcon />{t("app.submit.openPr")}</button>
          <button className="btn play" onClick={onClose}>{t("app.close")}</button>
        </div>
      </>
    );
  } else {
    const ok = checked && !checked.problems.length;
    content = (
      <>
        <p className="set-note">{t("app.submit.intro")}</p>
        <form className="sub-row" onSubmit={(e) => { e.preventDefault(); check(); }}>
          <input ref={input} value={repo} placeholder="https://github.com/owner/repository" spellCheck={false}
            onChange={(e) => { setRepo(e.target.value); setChecked(null); }} aria-label={t("app.submit.repo")} />
          <button type="submit" className="btn" disabled={!repo.trim() || checking}>{checking ? <span className="upd-spin" aria-hidden="true" /> : null}{t("app.submit.check")}</button>
        </form>
        {checked && checked.problems.length > 0 && (
          <ul className="sub-problems" aria-label={t("app.submit.problems")}>
            {checked.problems.map((p, i) => <li key={i}><WarnIcon />{problemText(p)}</li>)}
          </ul>
        )}
        {checked?.preview && (
          <div className={ok ? "sub-preview ok" : "sub-preview"}>
            <b>{ok && <CheckIcon />}{checked.preview.name}<small>{checked.preview.version} · {t(`app.scope.${checked.preview.scope}`)}</small></b>
            <p>{checked.preview.summary}</p>
          </div>
        )}
        {ok && (
          <label className="rf-field">
            <span className="rf-label">{t("app.submit.note")}<small>{t("app.submit.noteHint")}</small></span>
            <textarea value={note} rows={3} maxLength={2000} onChange={(e) => setNote(e.target.value)} />
          </label>
        )}
        <div className="modal-foot">
          <button data-pad-back className="btn" onClick={onClose}><PadGlyph role="east" />{t("app.cancel")}</button>
          <button className="btn play" onClick={send} disabled={!ok || sending}>{t("app.submit.send")}</button>
        </div>
      </>
    );
  }
  return createPortal(
    <div className="modal-back" onPointerDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="modal report-dialog" role="dialog" aria-modal="true" aria-label={t("app.submit.heading")}>
        <h2 className="cap">{t("app.submit.heading")}</h2>
        {content}
      </div>
    </div>,
    document.body,
  );
}

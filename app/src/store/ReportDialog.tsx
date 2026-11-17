// Report an issue: the issue templates of the mod repository to pick from (or a blank report when
// there are none, or when the repository allows one), the form or the text to fill in, and the
// problem report attached. The issue is filed as the signed-in player (core/src/github.rs).
import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { call, errorText } from "../api";
import { withTask } from "../busy";
import { setReports, signIn, useGitHub, type Report, type Templates } from "../github";
import { CheckIcon, ChevronIcon, GitHubIcon } from "../Icons";
import { t } from "../i18n";
import { notify } from "../notices";
import { useLayer } from "../Modal";
import { initial, formBody, missing, readTemplate, type Answers, type Field, type Template } from "./issueForms";
import type { StoreMod } from "./storeIndex";
import "./github.css";
import { PadGlyph } from "../pad/PadGlyph";

const BLANK: Template = { file: "", name: "", about: "", title: "", labels: [], fields: [], body: "" };

function FieldInput({ f, value, onChange }: { f: Field; value: Answers[string]; onChange: (v: Answers[string]) => void }) {
  if (f.type === "markdown") return <p className="rf-note">{f.description}</p>;
  const head = (
    <span className="rf-label">
      <span className="rf-name">{f.label}{f.required && <i className="rf-req" aria-label={t("app.report.required")}>*</i>}</span>
      {f.description && <small>{f.description}</small>}
    </span>
  );
  if (f.type === "checkboxes") {
    const v = (value as boolean[]) ?? [];
    return (
      <fieldset className="rf-field">
        <legend>{head}</legend>
        {f.options.map((o, i) => (
          <button key={i} type="button" role="checkbox" aria-checked={!!v[i]} className="check-line" onClick={() => onChange(v.map((x, j) => (j === i ? !x : x)))}>
            <span className={v[i] ? "ck on" : "ck"}>{v[i] && <CheckIcon />}</span>{o.label}{o.required && <i className="rf-req">*</i>}
          </button>
        ))}
      </fieldset>
    );
  }
  if (f.type === "dropdown") {
    const v = (value as string[]) ?? [];
    return (
      <fieldset className="rf-field">
        <legend>{head}</legend>
        <div className="rf-choices">
          {f.options.map((o) => {
            const on = v.includes(o.label);
            return (
              <button key={o.label} type="button" className={on ? "chip on" : "chip"} aria-pressed={on}
                onClick={() => onChange(f.multiple ? (on ? v.filter((x) => x !== o.label) : [...v, o.label]) : on ? [] : [o.label])}>
                {o.label}
              </button>
            );
          })}
        </div>
      </fieldset>
    );
  }
  return (
    <label className="rf-field">
      {head}
      {f.type === "input"
        ? <input value={String(value ?? "")} placeholder={f.placeholder} onChange={(e) => onChange(e.target.value)} />
        : <textarea value={String(value ?? "")} placeholder={f.placeholder} rows={f.render ? 6 : 4} className={f.render ? "mono" : undefined} onChange={(e) => onChange(e.target.value)} />}
    </label>
  );
}

export function ReportDialog({ m, onClose }: { m: StoreMod; onClose: () => void }) {
  const { account, signing } = useGitHub();
  const [list, setList] = useState<{ templates: Template[]; blank: boolean; open: boolean } | null>(null);
  const [failed, setFailed] = useState<string | null>(null);
  const [pick, setPick] = useState<Template | null>(null);
  const [title, setTitle] = useState("");
  const [body, setBody] = useState("");
  const [answers, setAnswers] = useState<Answers>({});
  const [attach, setAttach] = useState(true);
  const [sending, setSending] = useState(false);
  const [tried, setTried] = useState(false);
  const first = useRef<HTMLInputElement>(null);

  useLayer(() => { if (!pick) onClose(); });

  useEffect(() => {
    if (!account) return;
    let live = true;
    call<Templates>("github_templates", { repo: m.repo })
      .then((got) => {
        if (!live) return;
        const templates = got.files.map((f) => readTemplate(f.name, f.text)).filter((x): x is Template => !!x);
        setList({ templates, blank: got.blank || !templates.length, open: got.hasIssues });
        if (got.hasIssues && !templates.length) choose(BLANK);
      })
      .catch((e) => live && setFailed(errorText(e)));
    return () => { live = false; };
  }, [account, m.repo]);

  function choose(tp: Template) {
    setPick(tp);
    setTitle(tp.title);
    setBody(tp.body ?? "");
    setAnswers(Object.fromEntries(tp.fields.map((f) => [f.id, initial(f)])));
    setTried(false);
    setTimeout(() => first.current?.focus(), 0);
  }

  const lacking = pick ? missing(pick, answers) : [];
  const titleOk = title.trim().length >= 4;
  const send = async () => {
    setTried(true);
    if (!pick || !titleOk || lacking.length || sending) return;
    setSending(true);
    try {
      let text = pick.fields.length ? formBody(pick, answers) : body.trim();
      if (attach) {
        const report = await call<string>("github_diagnostics");
        text += `\n\n<details><summary>Obsidian report</summary>\n\n\`\`\`\n${report.trim()}\n\`\`\`\n</details>`;
      }
      text += `\n\n<sub>Sent from Obsidian Launcher, ${m.name} ${m.version}</sub>`;
      const rep = await withTask(t("app.report.sending"), () =>
        call<Report>("github_report", { repo: m.repo, title: title.trim(), body: text, labels: pick.labels, modId: m.id, modName: m.name }));
      setReports(await call<Report[]>("github_reports"));
      notify("info", t("app.report.sent", rep.number));
      onClose();
    } catch (e) {
      notify("error", errorText(e));
      setSending(false);
    }
  };

  let content;
  if (!account) {
    content = (
      <>
        <p>{t("app.report.signInText")}</p>
        <div className="modal-foot">
          <button data-pad-back className="btn" onClick={onClose}><PadGlyph role="east" />{t("app.cancel")}</button>
          <button className="btn play" onClick={() => signIn()} disabled={!!signing}><GitHubIcon />{signing ? t("app.report.signingIn") : t("app.report.signIn")}</button>
        </div>
        {signing && <p className="rf-code">{t("app.report.typeCode")} <b className="mono">{signing.userCode}</b></p>}
      </>
    );
  } else if (failed) {
    content = <><p>{failed}</p><div className="modal-foot"><button className="btn" onClick={onClose}>{t("app.close")}</button></div></>;
  } else if (!list) {
    content = <p><span className="upd-spin" aria-hidden="true" />{t("app.report.loading")}</p>;
  } else if (!list.open) {
    content = <><p>{t("app.report.closedRepo")}</p><div className="modal-foot"><button className="btn" onClick={onClose}>{t("app.close")}</button></div></>;
  } else if (!pick) {
    content = (
      <>
        <ul className="rf-pick">
          {list.templates.map((tp) => (
            <li key={tp.file}>
              <button onClick={() => choose(tp)}><b>{tp.name}</b>{tp.about && <small>{tp.about}</small>}<ChevronIcon /></button>
            </li>
          ))}
          {list.blank && (
            <li><button onClick={() => choose(BLANK)}><b>{t("app.report.blank")}</b><small>{t("app.report.blankHint")}</small><ChevronIcon /></button></li>
          )}
        </ul>
        <div className="modal-foot"><button data-pad-back className="btn" onClick={onClose}><PadGlyph role="east" />{t("app.cancel")}</button></div>
      </>
    );
  } else {
    content = (
      <form className="rf-form" onSubmit={(e) => { e.preventDefault(); send(); }}>
        <div className="rf-scroll">
          <label className={tried && !titleOk ? "rf-field bad" : "rf-field"}>
            <span className="rf-label"><span className="rf-name">{t("app.report.title")}<i className="rf-req" aria-label={t("app.report.required")}>*</i></span></span>
            <input ref={first} value={title} maxLength={200} onChange={(e) => setTitle(e.target.value)} />
          </label>
          {pick.fields.length
            ? pick.fields.map((f) => (
              <div key={f.id} className={tried && lacking.includes(f) ? "bad" : undefined}>
                <FieldInput f={f} value={answers[f.id]} onChange={(v) => setAnswers((a) => ({ ...a, [f.id]: v }))} />
              </div>
            ))
            : (
              <label className="rf-field">
                <span className="rf-label">{t("app.report.description")}</span>
                <textarea value={body} rows={10} onChange={(e) => setBody(e.target.value)} placeholder={t("app.report.descriptionHint")} />
              </label>
            )}
          <button type="button" role="checkbox" aria-checked={attach} className="check-line rf-attach" onClick={() => setAttach(!attach)}>
            <span className={attach ? "ck on" : "ck"}>{attach && <CheckIcon />}</span>
            <span>{t("app.report.attach")}<small>{t("app.report.attachHint")}</small></span>
          </button>
        </div>
        {tried && (lacking.length > 0 || !titleOk) && <p className="rf-missing">{t("app.report.missing")}</p>}
        <div className="modal-foot">
          {list.templates.length > 0 && <button type="button" className="link foot-link" onClick={() => setPick(null)}>{t("app.report.otherKind")}</button>}
          <button data-pad-back type="button" className="btn" onClick={onClose}><PadGlyph role="east" />{t("app.cancel")}</button>
          <button type="submit" className="btn play" disabled={sending}>{t("app.report.send")}</button>
        </div>
      </form>
    );
  }

  return createPortal(
    <div className="modal-back" onPointerDown={(e) => e.target === e.currentTarget && !pick && onClose()}>
      <div className="modal report-dialog" role="dialog" aria-modal="true" aria-label={t("app.report.heading", m.name)}>
        <h2 className="cap">{pick?.name ? `${t("app.report.heading", m.name)}: ${pick.name}` : t("app.report.heading", m.name)}</h2>
        <p className="rf-as">{account ? t("app.report.as", account.login, m.repo) : null}</p>
        {content}
      </div>
    </div>,
    document.body,
  );
}

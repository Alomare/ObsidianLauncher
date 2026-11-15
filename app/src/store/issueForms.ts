// GitHub issue templates, read the way github.com reads them: YAML issue forms (`.yml`) with fields,
// and Markdown templates (`.md`) with front matter and a body to fill in. A report is written back as
// github.com writes a form: one `### Label` section per field.
import { parse } from "yaml";

export type FieldType = "markdown" | "textarea" | "input" | "dropdown" | "checkboxes";
export type Field = {
  type: FieldType;
  id: string;
  label: string;
  description: string;
  placeholder: string;
  value: string;
  required: boolean;
  /** dropdown: the choices; checkboxes: the boxes (`required` per box). */
  options: { label: string; required: boolean }[];
  multiple: boolean;
  /** textarea: the language its text is shown in, as code. */
  render: string;
};
export type Template = { file: string; name: string; about: string; title: string; labels: string[]; fields: Field[]; body: string | null };

const str = (v: unknown) => (typeof v === "string" ? v : typeof v === "number" ? String(v) : "");
const labelsOf = (v: unknown) =>
  (Array.isArray(v) ? v.map(str) : str(v).split(",")).map((l) => l.trim()).filter(Boolean);

function field(raw: unknown, i: number): Field | null {
  if (!raw || typeof raw !== "object") return null;
  const r = raw as Record<string, unknown>;
  const type = str(r.type) as FieldType;
  if (!["markdown", "textarea", "input", "dropdown", "checkboxes"].includes(type)) return null;
  const a = (r.attributes ?? {}) as Record<string, unknown>;
  const v = (r.validations ?? {}) as Record<string, unknown>;
  const options = Array.isArray(a.options)
    ? a.options.map((o) => (typeof o === "object" && o ? { label: str((o as Record<string, unknown>).label), required: !!(o as Record<string, unknown>).required } : { label: str(o), required: false }))
    : [];
  return {
    type,
    id: str(r.id) || `field-${i}`,
    label: str(a.label),
    description: str(a.description),
    placeholder: str(a.placeholder),
    value: type === "markdown" ? "" : str(a.value),
    required: !!v.required,
    options,
    multiple: !!a.multiple,
    render: str(a.render),
  } satisfies Field;
}

/** One template file, or null when it can't be read (GitHub would skip it too). */
export function readTemplate(file: string, text: string): Template | null {
  try {
    if (/\.ya?ml$/i.test(file)) {
      const y = parse(text) as Record<string, unknown> | null;
      if (!y || typeof y !== "object" || !Array.isArray(y.body)) return null;
      const fields = y.body.map(field).filter((f): f is Field => !!f);
      // Markdown blocks carry their text in `value`.
      y.body.forEach((b: Record<string, unknown>, i: number) => {
        const f = fields.find((x) => x.id === (str(b?.id) || `field-${i}`));
        if (f && f.type === "markdown") f.description = str((b?.attributes as Record<string, unknown>)?.value);
      });
      return { file, name: str(y.name) || file, about: str(y.description), title: str(y.title), labels: labelsOf(y.labels), fields, body: null };
    }
    const m = /^---\r?\n([\s\S]*?)\r?\n---\r?\n?([\s\S]*)$/.exec(text);
    const head = m ? ((parse(m[1]) ?? {}) as Record<string, unknown>) : {};
    return { file, name: str(head.name) || file.replace(/\.md$/i, ""), about: str(head.about), title: str(head.title), labels: labelsOf(head.labels), fields: [], body: m ? m[2] : text };
  } catch {
    return null;
  }
}

export type Answers = Record<string, string | string[] | boolean[]>;

/** What a field starts as. */
export function initial(f: Field): string | string[] | boolean[] {
  if (f.type === "checkboxes") return f.options.map(() => false);
  if (f.type === "dropdown") return [];
  return f.value;
}

/** The fields still missing an answer GitHub would ask for. */
export function missing(t: Template, a: Answers): Field[] {
  return t.fields.filter((f) => {
    const v = a[f.id];
    if (f.type === "checkboxes") return f.options.some((o, i) => o.required && !(v as boolean[] | undefined)?.[i]);
    if (!f.required) return false;
    if (f.type === "dropdown") return !(v as string[] | undefined)?.length;
    return !String(v ?? "").trim();
  });
}

/** The issue body, as github.com writes a filled form. */
export function formBody(t: Template, a: Answers): string {
  const out: string[] = [];
  for (const f of t.fields) {
    if (f.type === "markdown") continue;
    const v = a[f.id];
    let text: string;
    if (f.type === "checkboxes") {
      text = f.options.map((o, i) => `- [${(v as boolean[] | undefined)?.[i] ? "X" : " "}] ${o.label}`).join("\n");
    } else if (f.type === "dropdown") {
      text = ((v as string[] | undefined) ?? []).join(", ");
    } else {
      text = String(v ?? "").trim();
      if (text && f.type === "textarea" && f.render) text = "```" + f.render + "\n" + text + "\n```";
    }
    out.push(`### ${f.label}\n\n${text || "_No response_"}`);
  }
  return out.join("\n\n");
}

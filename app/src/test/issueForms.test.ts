import { describe, expect, it } from "vitest";
import { formBody, initial, missing, readTemplate, type Answers } from "../store/issueForms";
import { sortMods } from "../store/StorePage";
import type { StoreMod } from "../store/storeIndex";

const FORM = `
name: Bug report
description: Something does not work
title: "[Bug]: "
labels: [bug, triage]
body:
  - type: markdown
    attributes:
      value: Thanks for the report.
  - type: input
    id: version
    attributes:
      label: Mod version
    validations:
      required: true
  - type: dropdown
    id: where
    attributes:
      label: Where
      options: [Menu, Mission]
      multiple: true
  - type: textarea
    id: log
    attributes:
      label: Log
      render: shell
  - type: checkboxes
    id: terms
    attributes:
      label: Checks
      options:
        - label: Searched existing issues
          required: true
        - label: Optional box
`;

describe("issue templates", () => {
  it("reads a YAML form with its fields, labels and title", () => {
    const tp = readTemplate("bug.yml", FORM)!;
    expect(tp.name).toBe("Bug report");
    expect(tp.title).toBe("[Bug]: ");
    expect(tp.labels).toEqual(["bug", "triage"]);
    expect(tp.fields.map((f) => f.type)).toEqual(["markdown", "input", "dropdown", "textarea", "checkboxes"]);
    expect(tp.fields[0].description).toBe("Thanks for the report.");
  });

  it("asks for required fields and writes the body as github.com does", () => {
    const tp = readTemplate("bug.yml", FORM)!;
    const a: Answers = Object.fromEntries(tp.fields.map((f) => [f.id, initial(f)]));
    expect(missing(tp, a).map((f) => f.id)).toEqual(["version", "terms"]);
    a.version = "1.2.0";
    a.where = ["Mission"];
    a.log = "boom";
    a.terms = [true, false];
    expect(missing(tp, a)).toEqual([]);
    expect(formBody(tp, a)).toBe(
      "### Mod version\n\n1.2.0\n\n### Where\n\nMission\n\n### Log\n\n```shell\nboom\n```\n\n### Checks\n\n- [X] Searched existing issues\n- [ ] Optional box",
    );
  });

  it("reads a Markdown template with front matter, and skips broken files", () => {
    const tp = readTemplate("feature.md", "---\nname: Feature\nabout: An idea\ntitle: ''\nlabels: enhancement, idea\n---\n## Idea\n")!;
    expect(tp).toMatchObject({ name: "Feature", about: "An idea", labels: ["enhancement", "idea"], body: "## Idea\n", fields: [] });
    expect(readTemplate("bad.yml", "body: [unclosed")).toBeNull();
    expect(readTemplate("nobody.yml", "name: x")).toBeNull();
  });
});

describe("stars", () => {
  it("sorts the most starred first", () => {
    const mod = (id: string, stars?: number) => ({ id, name: id, stars }) as StoreMod;
    expect(sortMods([mod("a", 1), mod("b", 9), mod("c")], "stars", []).map((m) => m.id)).toEqual(["b", "a", "c"]);
  });
});

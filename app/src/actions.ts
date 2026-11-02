// Deploy, Play and Purge (spec section 5). Each shows what it is doing in the status bar and ends
// with a notice.
import { call, errorText, setSnapshot } from "./api";
import { withTask } from "./busy";
import { t } from "./i18n";
import { notify } from "./notices";
import type { Snapshot } from "./types";

type DeployDone = { snapshot: Snapshot; mods: number; backup: boolean; unchanged: boolean };
type PurgeDone = { snapshot: Snapshot; removed: number; kept: number };

async function step<T>(busy: string, work: () => Promise<T>): Promise<T | null> {
  try {
    return await withTask(busy, work, true);
  } catch (e) {
    notify("error", errorText(e));
    return null;
  }
}

export async function deploy() {
  const r = await step(t("app.action.deploying"), () => call<DeployDone>("deploy"));
  if (!r) return;
  setSnapshot(r.snapshot);
  if (r.unchanged) return notify("info", t("app.action.deployUpToDate"));
  notify("info", r.mods === 1 ? t("app.action.deployedOne") : r.mods ? t("app.action.deployed", r.mods) : t("app.action.deployedVanilla"));
}

export async function purge() {
  const r = await step(t("app.action.purging"), () => call<PurgeDone>("purge"));
  if (!r) return;
  setSnapshot(r.snapshot);
  notify("info", r.kept ? t("app.action.purgedKept", r.kept) : t("app.action.purged"));
}

export async function play() {
  const s = await step(t("app.action.starting"), () => call<Snapshot>("play"));
  if (!s) return;
  setSnapshot(s);
  notify("info", t("app.action.started"));
}

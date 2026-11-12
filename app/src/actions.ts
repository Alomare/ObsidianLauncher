// Deploy, Play and Purge (spec section 5). Each shows what it is doing in the status bar and ends
// with a notice.
import { call, errorText, setSnapshot } from "./api";
import { withTask } from "./busy";
import { t } from "./i18n";
import { notify } from "./notices";
import type { CmdError, Snapshot } from "./types";

type DeployDone = { snapshot: Snapshot; mods: number; backup: boolean; unchanged: boolean; settings: boolean; loader: boolean };
type PurgeDone = { snapshot: Snapshot; removed: number; kept: number };

async function step<T>(busy: string, work: () => Promise<T>): Promise<T | null> {
  try {
    return await withTask(busy, work, true);
  } catch (e) {
    notify("error", errorText(e));
    return null;
  }
}

/** Whether the deploy went through. */
export async function deploy(): Promise<boolean> {
  const r = await step(t("app.action.deploying"), () => call<DeployDone>("deploy"));
  if (!r) return false;
  setSnapshot(r.snapshot);
  if (r.unchanged) notify("info", t(r.settings ? "app.action.deployedOptions" : "app.action.deployUpToDate"));
  else notify("info", r.mods === 1 ? t("app.action.deployedOne") : r.mods ? t("app.action.deployed", r.mods) : t(r.loader ? "app.action.deployedLoader" : "app.action.deployedVanilla"));
  return true;
}

/** The Saves tab's deploy: puts the loader in when it is missing, which a Sandbox save needs, and leaves the deployed mods as they are. */
export async function deployLoader(): Promise<boolean> {
  const r = await step(t("app.action.deploying"), () => call<DeployDone>("deploy_loader"));
  if (!r) return false;
  setSnapshot(r.snapshot);
  notify("info", t(r.unchanged ? "app.action.loaderPresent" : "app.action.deployedLoader"));
  return true;
}

export async function purge() {
  const r = await step(t("app.action.purging"), () => call<PurgeDone>("purge"));
  if (!r) return;
  setSnapshot(r.snapshot);
  notify("info", r.kept ? t("app.action.purgedKept", r.kept) : t("app.action.purged"));
}

/** `needsDeploy`: a Sandbox save is selected and the loader is not in the game; the caller asks to deploy. */
export async function play(): Promise<"ok" | "needsDeploy" | "failed"> {
  try {
    const s = await withTask(t("app.action.starting"), () => call<Snapshot>("play"), true);
    setSnapshot(s);
    notify("info", t("app.action.started"));
    return "ok";
  } catch (e) {
    if ((e as CmdError)?.code === "SandboxNeedsDeploy") return "needsDeploy";
    notify("error", errorText(e));
    return "failed";
  }
}

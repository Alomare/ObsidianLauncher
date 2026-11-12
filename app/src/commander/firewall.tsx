// LAN wars through a Linux firewall. Windows asks the player by itself the first time a program
// listens on the network; firewalld and ufw block the ports without asking. Obsidian opens the
// Commander ports only when the player agrees, with the system password prompt, and Remove All
// Commander Data closes them again (core/src/commander/firewall.rs).
import { useCallback, useEffect, useState } from "react";
import { call, errorText } from "../api";
import { withTask } from "../busy";
import { Confirm } from "../Confirm";
import { t } from "../i18n";
import { notify } from "../notices";

export type FirewallState = { linux: boolean; backend: "firewalld" | "ufw" | null; open: boolean; recorded: boolean };
type Outcome = { kind: "done" | "declined" | "failed"; message?: string };

let known: FirewallState | null = null;

/** The firewall state, read once a session and again after a change. `blocked`: a firewall is on and the LAN ports are shut. */
export function useFirewall() {
  const [fw, setFw] = useState<FirewallState | null>(known);
  const refresh = useCallback(async () => {
    try {
      known = await call<FirewallState>("commander_firewall");
      setFw(known);
    } catch { /* only a hint: without it nothing is offered */ }
  }, []);
  useEffect(() => { if (!known) void refresh(); }, [refresh]);
  return { fw, refresh, blocked: !!fw?.backend && !fw.open };
}

export function FirewallDialog({ backend, onClose, onDone }: { backend: string; onClose: () => void; onDone: () => void }) {
  const allow = async () => {
    onClose();
    try {
      const r = await withTask(t("app.commander.firewallOpening"), () => call<Outcome>("commander_firewall_open"));
      if (r.kind === "done") notify("info", t("app.commander.firewallOpened"));
      else if (r.kind === "declined") notify("info", t("app.commander.firewallDeclined"));
      else notify("error", t("app.commander.firewallFailed", r.message ?? ""));
    } catch (e) { notify("error", errorText(e)); }
    onDone();
  };
  return <Confirm title={t("app.commander.firewallTitle")} text={t("app.commander.firewallText", backend)} confirm={t("app.commander.firewallAllow")} tone="go" onConfirm={allow} onCancel={onClose} />;
}

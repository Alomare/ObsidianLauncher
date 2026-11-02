// Times the dev window's interactions with the CPU slowed down, the way a weak PC would run them.
// Each interaction runs through real input (DevTools Input events) and is timed inside the page,
// from the input event to the second frame after the result is on screen.
//
//   node tools/perf.mjs [rates] [runs]      e.g. node tools/perf.mjs 1,4,6 5
//
// Needs the dev app on port 9222 (see tools/cdp.mjs) on the Mods tab with at least 4 mods. For a
// large library, start the app with OBSIDIAN_DATA_DIR pointing at a folder made by
//   cargo run -p obsidian-app --example perf_library -- <dir> 200
// The library is left as it was: every change an interaction makes is undone.
const [rates = "1,4,6", runsArg = "5"] = process.argv.slice(2);
const RATES = rates.split(",").map(Number);
const RUNS = +runsArg;
// Interactions should answer within 100 ms on a 4x slower CPU; dragging should keep 30 fps.
const BUDGET_MS = 100;
const FRAME_BUDGET_MS = 33.4;

const all = await (await fetch("http://127.0.0.1:9222/json")).json();
const target = all.find((t) => t.type === "page" && t.url.includes("localhost:1420")) ?? all.find((t) => t.type === "page");
if (!target) throw new Error("no page on port 9222");
const ws = new WebSocket(target.webSocketDebuggerUrl);
await new Promise((ok, fail) => { ws.onopen = ok; ws.onerror = fail; });
let next = 1;
const pending = new Map();
ws.onmessage = (m) => {
  const msg = JSON.parse(m.data);
  if (msg.id && pending.has(msg.id)) { pending.get(msg.id)(msg); pending.delete(msg.id); }
};
const send = (method, params = {}) => new Promise((ok) => { const id = next++; pending.set(id, ok); ws.send(JSON.stringify({ id, method, params })); });
async function js(expression) {
  const r = await send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
  if (r.result?.exceptionDetails) throw new Error(r.result.exceptionDetails.exception?.description ?? r.result.exceptionDetails.text);
  return r.result?.result?.value;
}
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// Page side: arm() waits for the next input event, then for `done()` to hold, then one more frame.
await js(`(() => {
  const frame = () => new Promise((r) => requestAnimationFrame(() => r()));
  window.__perf = {
    frame,
    arm(done) {
      let t0 = null;
      const mark = (e) => { if (t0 === null) t0 = e.timeStamp; };
      for (const t of ["pointerdown", "keydown", "contextmenu", "input"]) window.addEventListener(t, mark, { capture: true, once: true });
      window.__perf.p = (async () => {
        const start = performance.now();
        while (t0 === null || !done()) {
          if (performance.now() - start > 10000) throw new Error("timed out");
          await frame();
        }
        await frame();
        return performance.now() - t0;
      })();
    },
    async timed(work) {
      const t0 = performance.now();
      await work();
      await frame(); await frame();
      return performance.now() - t0;
    },
    frames() {
      const gaps = []; let last = performance.now(); let on = true; let long = 0;
      const obs = new PerformanceObserver((l) => { long += l.getEntries().length; });
      try { obs.observe({ entryTypes: ["longtask"] }); } catch {}
      const tick = (t) => { gaps.push(t - last); last = t; if (on) requestAnimationFrame(tick); };
      requestAnimationFrame(tick);
      window.__perf.stop = () => { on = false; obs.disconnect(); return { gaps: gaps.slice(1), long }; };
    },
  };
  return 1;
})()`);

const center = (sel) => js(`(() => { const r = document.querySelector(${JSON.stringify(sel)}).getBoundingClientRect(); return [r.x + r.width / 2, r.y + r.height / 2]; })()`);
/** A mouse event; `held` says whether the button stays down (a press, or a move while dragging). */
const mouse = (type, x, y, button = "left", held = type !== "mouseReleased") =>
  send("Input.dispatchMouseEvent", { type, x, y, button: held || type === "mouseReleased" ? button : "none", buttons: held ? (button === "left" ? 1 : 2) : 0, clickCount: 1 });
async function click(sel, button = "left") {
  const [x, y] = await center(sel);
  await mouse("mouseMoved", x, y, button, false);
  await mouse("mousePressed", x, y, button);
  await mouse("mouseReleased", x, y, button);
}
const key = async (k, code = k) => {
  await send("Input.dispatchKeyEvent", { type: "keyDown", key: k, code, windowsVirtualKeyCode: k === "Escape" ? 27 : 0 });
  await send("Input.dispatchKeyEvent", { type: "keyUp", key: k, code, windowsVirtualKeyCode: k === "Escape" ? 27 : 0 });
};
/** Arms the page, does `act`, and returns the time from the input to the result on screen. */
async function measure(done, act) {
  await js(`window.__perf.arm(() => ${done})`);
  await act();
  return js("window.__perf.p");
}

const invoke = (cmd, args = {}) => js(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(cmd)}, ${JSON.stringify(args)})`);
// Home, Mods and Wiki are the tabs on the left; Settings is the gear at the far right of the bar.
const tab = (i) => (i === SETTINGS ? `.nav > .tabs:last-child .tab` : `.nav > .tabs:first-child .tab:nth-child(${i})`);
const SETTINGS = 4;
// The nth mod row that can move (not in a locked folder).
const rowSel = (n) => js(`[...document.querySelectorAll(".tbody .tr[data-key]:not(.fr):not(.pinned)")][${n}]?.dataset.key ?? null`);

const SCENARIOS = {
  "tab: to Settings": () => measure(`!!document.querySelector(".settings")`, () => click(tab(SETTINGS))),
  "tab: to Mods": async () => {
    await click(tab(SETTINGS));
    await sleep(150);
    return measure(`!!document.querySelector(".mods .tbody")`, () => click(tab(2)));
  },
  "drawer: open a mod": async () => {
    const id = await rowSel(2);
    const sel = `[data-key="${id}"]`;
    return measure(`!!document.querySelector('${sel}.sel')`, () => click(`${sel} .who`));
  },
  "toggle: on/off round trip": async () => {
    const id = await rowSel(2);
    const sw = `[data-key="${id}"] .sw`;
    const was = await js(`document.querySelector('${sw}').getAttribute("aria-checked")`);
    const ms = await measure(`document.querySelector('${sw}').getAttribute("aria-checked") !== "${was}"`, () => click(sw));
    await measure(`document.querySelector('${sw}').getAttribute("aria-checked") === "${was}"`, () => click(sw));
    return ms;
  },
  "search: type a letter": async () => {
    await js(`document.querySelector(".search input").focus()`);
    const ms = await measure(`document.querySelector(".search input").value === "e"`, () => send("Input.insertText", { text: "e" }));
    await key("Escape");
    await sleep(50);
    return ms;
  },
  "menu: right click": async () => {
    const id = await rowSel(2);
    const ms = await measure(`!!document.querySelector(".ctx")`, () => click(`[data-key="${id}"] .who`, "right"));
    await key("Escape");
    return ms;
  },
  "profiles: open dialog": async () => {
    const ms = await measure(`!!document.querySelector(".modal.profiles")`, () => click(".profile-btn"));
    await key("Escape");
    await sleep(50);
    return ms;
  },
  "language: switch and back": async () => {
    const was = await js(`window.__obsidian.settings().language`);
    const ms = await js(`window.__perf.timed(() => window.__obsidian.updateSettings({ language: ${JSON.stringify(was === "pt-BR" ? "en" : "pt-BR")} }))`);
    await js(`window.__obsidian.updateSettings({ language: ${JSON.stringify(was)} })`);
    return ms;
  },
};

/** Drags the third movable row up two places in small steps, recording the frame gaps. */
async function dragFrames() {
  const before = (await invoke("snapshot")).mods.map((m) => m.id);
  const id = await rowSel(3);
  const up = await rowSel(1);
  const [x, y1] = await center(`[data-key="${id}"] .who`);
  const [, y2] = await center(`[data-key="${up}"] .who`);
  await js("window.__perf.frames()");
  await mouse("mousePressed", x, y1);
  for (let i = 1; i <= 30; i++) {
    await mouse("mouseMoved", x, y1 + ((y2 - y1) * i) / 30);
    await sleep(16);
  }
  await mouse("mouseReleased", x, y2);
  await sleep(300);
  const r = await js("window.__perf.stop()");
  await invoke("set_order", { ids: before, moves: null });
  return r;
}

const median = (a) => [...a].sort((x, y) => x - y)[Math.floor(a.length / 2)];
const pct = (a, p) => [...a].sort((x, y) => x - y)[Math.min(a.length - 1, Math.floor(a.length * p))];
const fmt = (n) => n.toFixed(0).padStart(5);

if (!(await js(`!!window.__obsidian`))) throw new Error("window.__obsidian is missing: run the dev build (npm run tauri dev)");
await click(tab(2));
await sleep(300);
const mods = (await invoke("snapshot")).mods.length;
console.log(`Obsidian UI timings: ${mods} mods, ${RUNS} runs each, median / worst in ms (budget ${BUDGET_MS} ms)\n`);
const results = [];
for (const rate of RATES) {
  await send("Emulation.setCPUThrottlingRate", { rate });
  console.log(`CPU ${rate}x slower`);
  for (const [name, run] of Object.entries(SCENARIOS)) {
    const times = [];
    for (let i = 0; i < RUNS; i++) {
      times.push(await run());
      if (name.startsWith("tab: to Settings")) await click(tab(2));
      await sleep(80);
    }
    const m = median(times);
    const flag = rate <= 4 && m > BUDGET_MS ? "  OVER" : "";
    console.log(`  ${name.padEnd(28)} ${fmt(m)} ${fmt(Math.max(...times))}${flag}`);
    results.push({ rate, name, median: m, worst: Math.max(...times) });
  }
  const d = await dragFrames();
  const p95 = pct(d.gaps, 0.95);
  const flag = rate <= 4 && p95 > FRAME_BUDGET_MS ? "  OVER" : "";
  console.log(`  ${"drag: frame gap p95".padEnd(28)} ${fmt(p95)} ${fmt(Math.max(...d.gaps))}  (${d.gaps.length} frames, ${d.long} long tasks)${flag}`);
  results.push({ rate, name: "drag: frame gap p95", median: p95, worst: Math.max(...d.gaps) });
}
await send("Emulation.setCPUThrottlingRate", { rate: 1 });
const over = results.filter((r) => r.rate <= 4 && r.median > (r.name.startsWith("drag") ? FRAME_BUDGET_MS : BUDGET_MS));
console.log(over.length ? `\n${over.length} over budget at 4x or less` : "\nAll within budget at 4x or less");
ws.close();
process.exit(over.length ? 1 : 0);

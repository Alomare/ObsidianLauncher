// A dialog leaves the page the moment it closes, so it would vanish while it came in with a rise. This
// keeps a copy of a closed one on screen for a moment, fading and sinking (styles.css, .modal-back.leaving).
// The copy is inert and hidden from screen readers, so nothing can be clicked or read in it, and the
// dialogs themselves close as they always did.
const OUT_MS = 140;

export function fadeClosedDialogs(root: HTMLElement = document.body) {
  new MutationObserver((records) => {
    if (document.documentElement.dataset.motion === "off") return;
    for (const r of records) {
      for (const node of r.removedNodes) {
        if (!(node instanceof HTMLElement) || !node.classList.contains("modal-back") || node.classList.contains("leaving")) continue;
        const ghost = node.cloneNode(true) as HTMLElement;
        ghost.classList.add("leaving");
        ghost.inert = true;
        ghost.setAttribute("aria-hidden", "true");
        root.appendChild(ghost);
        setTimeout(() => ghost.remove(), OUT_MS);
      }
    }
  }).observe(root, { childList: true });
}

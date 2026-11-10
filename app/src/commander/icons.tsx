// Commander's icons. The tab icon: three fronts around Super Earth, the shape of the game's war map.
export const CommanderIcon = () => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    <circle cx="12" cy="12" r="2.6" />
    <path d="M5.2 8.1A8 8 0 0 1 10.6 4.1M13.4 4.1a8 8 0 0 1 5.4 4M19.6 10.6a8 8 0 0 1-2.4 7.1M14.6 19.6a8 8 0 0 1-5.2 0M6.8 17.7a8 8 0 0 1-2.4-7.1" />
    <path d="M7.9 9.6a4.8 4.8 0 0 1 2.6-2.3M13.5 7.3a4.8 4.8 0 0 1 2.6 2.3M16.6 13.1a4.8 4.8 0 0 1-2.1 3.1M9.5 16.2a4.8 4.8 0 0 1-2.1-3.1" />
  </svg>
);

/** A screen on a stand: the wars hosted on this PC. */
export const ThisPcIcon = () => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="square" aria-hidden="true">
    <rect x="3" y="4" width="18" height="12" />
    <path d="M12 16v4M7.5 20h9" />
  </svg>
);

/** A box: the Commander package and what can be done with it. */
export const PackageIcon = () => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinejoin="round" aria-hidden="true">
    <path d="M12 3l8 4.5v9L12 21l-8-4.5v-9z" />
    <path d="M4 7.5l8 4.5 8-4.5M12 12v9M8 5.3l8 4.5" />
  </svg>
);

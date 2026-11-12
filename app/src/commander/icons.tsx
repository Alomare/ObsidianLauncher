// The Commander icons. The tab icon: three fronts around Super Earth, the shape of the in-game war map.
export const CommanderIcon = () => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    <circle cx="12" cy="12" r="3.2" />
    <path d="M3.84 7.32A9.6 9.6 0 0 1 10.32 2.52M13.68 2.52A9.6 9.6 0 0 1 20.16 7.32M21.12 10.32A9.6 9.6 0 0 1 18.24 18.84M15.12 21.12A9.6 9.6 0 0 1 8.88 21.12M5.76 18.84A9.6 9.6 0 0 1 2.88 10.32" />
  </svg>
);

/** A screen on a stand: the wars hosted on this PC. */
export const ThisPcIcon = () => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="square" aria-hidden="true">
    <rect x="3" y="4" width="18" height="12" />
    <path d="M12 16v4M7.5 20h9" />
  </svg>
);

/** Three lines: the list of what Play can start. */
export const MenuIcon = () => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="square" aria-hidden="true">
    <path d="M5 7h14M5 12h14M5 17h14" />
  </svg>
);

/** A box: the Commander package and what can be done with it. */
export const PackageIcon = () => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinejoin="round" aria-hidden="true">
    <path d="M12 3l8 4.5v9L12 21l-8-4.5v-9z" />
    <path d="M4 7.5l8 4.5 8-4.5M12 12v9M8 5.3l8 4.5" />
  </svg>
);

/** A brick wall: the firewall and the LAN ports. */
export const WallIcon = () => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="square" aria-hidden="true">
    <rect x="3" y="5" width="18" height="14" />
    <path d="M3 9.7h18M3 14.3h18M9 5v4.7M15 5v4.7M6 9.7v4.6M12 9.7v4.6M18 9.7v4.6M9 14.3V19M15 14.3V19" />
  </svg>
);

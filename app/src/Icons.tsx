// Line icons drawn in currentColor, and the Obsidian logo.
import { useId, type ReactNode, type SVGProps } from "react";

const Icon = ({ children, ...p }: SVGProps<SVGSVGElement> & { children: ReactNode }) => (
  <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" {...p}>
    {children}
  </svg>
);


/** A wrench. */
export const ModsIcon = () => <Icon><path d="M14.6 6.4a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.4-3.4a5.6 5.6 0 0 1-7.6 7.3l-7 7a2.1 2.1 0 0 1-3-3l7-7a5.6 5.6 0 0 1 7.3-7.6z" /></Icon>;
/** A floppy disk: the Saves tab. */
export const SaveIcon = () => <Icon><path d="M4.5 3.5h12l3 3v14h-15z" /><path d="M8 3.5v5h8v-5M7.5 20.5v-6.5h9v6.5" /></Icon>;
export const EyeIcon = () => <Icon><path d="M2.5 12S6 5.5 12 5.5 21.5 12 21.5 12 18 18.5 12 18.5 2.5 12 2.5 12z" /><circle cx="12" cy="12" r="2.8" /></Icon>;
export const EyeOffIcon = () => <Icon><path d="M9.8 5.8A8.9 8.9 0 0 1 12 5.5c6 0 9.5 6.5 9.5 6.5a16.6 16.6 0 0 1-2.6 3.5M6.2 7.4A16 16 0 0 0 2.5 12S6 18.5 12 18.5a8.8 8.8 0 0 0 4.4-1.2" /><path d="M9.9 10a2.8 2.8 0 0 0 3.9 4" /><path d="M3.5 3.5l17 17" /></Icon>;
export const HomeIcon = () => <Icon><path d="M3.5 11L12 3.5l8.5 7.5" /><path d="M5.5 9.5v11h5v-6h3v6h5v-11" /></Icon>;
/** A shop front with an awning: the Store tab. */
export const StoreIcon = () => <Icon><path d="M4.5 3.5h15l2 5.5a2.6 2.6 0 0 1-4.9 1.2 2.6 2.6 0 0 1-4.6 0 2.6 2.6 0 0 1-4.6 0A2.6 2.6 0 0 1 2.5 9z" /><path d="M4.5 12v8.5h15V12M10 20.5v-5h4v5" /></Icon>;
/** A picture crossed out: a mod with no thumbnail. */
export const NoImageIcon = () => <Icon strokeWidth="1.5"><rect x="3.5" y="4.5" width="17" height="15" rx="1" /><circle cx="9" cy="10" r="1.6" /><path d="M20.5 16l-5-5-8.5 8.5M3 3l18 18" /></Icon>;
const FOLDER = "M3 6.5a1.5 1.5 0 0 1 1.5-1.5h4.8l2 2.2h8.2A1.5 1.5 0 0 1 21 8.7v9.8a1.5 1.5 0 0 1-1.5 1.5h-15A1.5 1.5 0 0 1 3 18.5z";
export const FolderIcon = () => <Icon><path d={FOLDER} /></Icon>;
/** A folder outline with a plus inside. */
export const FolderAddIcon = () => <Icon strokeWidth="2"><path d={FOLDER} /><path d="M12 10.2v6.6M8.7 13.5h6.6" /></Icon>;
/** Solid, to read as a state at small sizes. */
export const LockIcon = () => <Icon strokeWidth="2.4"><path d="M8 11V7.8a4 4 0 0 1 8 0V11" /><rect x="4.5" y="10.5" width="15" height="10.5" rx="1.5" fill="currentColor" stroke="none" /></Icon>;
export const UnlockIcon = () => <Icon strokeWidth="2.4"><path d="M8 11V7.8a4 4 0 0 1 7.6-1.7" /><rect x="4.5" y="10.5" width="15" height="10.5" rx="1.5" fill="currentColor" stroke="none" /></Icon>;
export const TrashIcon = () => <Icon><path d="M4 7h16M9.5 7V4.5h5V7M6.5 7l1 13h9l1-13M10 11v5.5M14 11v5.5" /></Icon>;
export const CheckIcon =() => <Icon strokeWidth="2.6"><path d="M5 12.5l4.5 4.5L19 7.5" /></Icon>;
/** The switch of a mod row, on and off. */
export const SwitchOnIcon = () => <Icon><rect x="2" y="6.5" width="20" height="11" rx="1" fill="currentColor" /><rect x="14" y="9.5" width="5" height="5" fill="var(--btn)" stroke="none" /></Icon>;
export const SwitchOffIcon = () => <Icon><rect x="2" y="6.5" width="20" height="11" rx="1" /><rect x="5" y="9.5" width="5" height="5" fill="currentColor" stroke="none" /></Icon>;
/** A filled square with an arrow leaving it: the link opens outside the app. */
/** A filled square with an arrow cut out. Each copy has a separate mask: a shared id breaks every copy
    while the first one sits in a hidden element. */
export const OutboundIcon = () => {
  const id = `outbound-${useId().replace(/[^A-Za-z0-9_-]/g, "")}`;
  return (
    <svg viewBox="0 0 24 24" width="14" height="14" aria-hidden="true" className="outbound">
      <defs><mask id={id}><rect width="24" height="24" fill="#fff" /><path d="M8.5 15.5l7-7M10 8.5h5.5V14" stroke="#000" strokeWidth="2.4" fill="none" strokeLinecap="round" strokeLinejoin="round" /></mask></defs>
      <rect x="2.5" y="2.5" width="19" height="19" rx="2.5" fill="currentColor" mask={`url(#${id})`} />
    </svg>
  );
};
export const GitHubIcon = () => (
  <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor" aria-hidden="true"><path d="M12 .3a12 12 0 0 0-3.8 23.38c.6.12.83-.26.83-.58l-.02-2.04c-3.34.73-4.04-1.6-4.04-1.6-.55-1.4-1.34-1.77-1.34-1.77-1.08-.74.09-.73.09-.73 1.2.09 1.84 1.24 1.84 1.24 1.07 1.83 2.8 1.3 3.49 1 .1-.78.42-1.31.76-1.6-2.67-.31-5.47-1.34-5.47-5.93 0-1.31.47-2.38 1.24-3.22-.14-.3-.54-1.52.1-3.18 0 0 1-.32 3.3 1.23a11.5 11.5 0 0 1 6 0c2.28-1.55 3.29-1.23 3.29-1.23.64 1.66.24 2.88.12 3.18a4.65 4.65 0 0 1 1.23 3.22c0 4.61-2.8 5.62-5.48 5.92.42.36.81 1.1.81 2.22l-.01 3.29c0 .32.21.7.82.58A12 12 0 0 0 12 .3" /></svg>
);
export const ProfilesIcon = () => <Icon><circle cx="12" cy="8" r="4" /><path d="M4 21c1.5-4 4.5-6 8-6s6.5 2 8 6" /></Icon>;
/** Two people: other modders can reach this profile's game. */
/** A shield, filled while it guards something. */
export const ShieldIcon = ({ filled = false }: { filled?: boolean }) => (
  <Icon><path d="M12 3l7.5 3v5.5c0 4.6-3.2 8.3-7.5 9.5-4.3-1.2-7.5-4.9-7.5-9.5V6z" fill={filled ? "currentColor" : "none"} /></Icon>
);
export const ModdersIcon = () => <Icon><circle cx="9" cy="8" r="3.5" /><path d="M2.5 20c.8-3.6 3.2-5.5 6.5-5.5s5.7 1.9 6.5 5.5M15.5 4.8a3.5 3.5 0 0 1 0 6.4M18 14.9c1.9.8 3 2.5 3.5 5.1" /></Icon>;
export const CogIcon = () => (
  <Icon>
    <circle cx="12" cy="12" r="3" />
    <path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z" />
  </Icon>
);
/** An eraser: Purge wipes what Obsidian wrote, back to vanilla. */
export const PurgeIcon = () => <Icon width="22.7" height="22.7" strokeWidth="2"><path d="M9 20.5L2.8 14.3a1 1 0 0 1 0-1.4l9.5-9.5a1 1 0 0 1 1.4 0l6.8 6.8a1 1 0 0 1 0 1.4L11.6 20.5z" /><path d="M7.5 8.6l8.4 8.4M9 20.5h12" /></Icon>;
/** A box with an arrow leaving it: the mods go out into the game. Drawn like Purge. */
// Wider than the other icons (3:2), so the arrow can run out to the right; `.icon-wide` keeps the shape.
export const DeployIcon = () => <Icon viewBox="0 0 36 24" width="36" height="24" strokeWidth="1.9" className="icon-wide"><path d="M8 4l7 4v8l-7 4-7-4V8z" /><path d="M1 8l7 4 7-4M8 12v8" /><path d="M18.5 12h16M32 9.25l2.75 2.75L32 14.75" /></Icon>;
export const PlayIcon = () => <Icon fill="currentColor" stroke="none"><path d="M7 4.5v15l13-7.5z" /></Icon>;
/** A triangle with a mark; `filled` makes it solid, with the mark cut out in the color of the panels. */
export const WarnIcon = ({ filled }: { filled?: boolean }) => (
  <Icon><path d="M12 3l10 18H2z" fill={filled ? "currentColor" : undefined} /><path d="M12 10v5M12 17.5v.5" stroke={filled ? "var(--s2)" : undefined} /></Icon>
);
export const SearchIcon = () => <Icon><circle cx="11" cy="11" r="7" /><path d="M20 20l-4-4" /></Icon>;
export const UpArrowIcon = () => <Icon strokeWidth="3"><path d="M12 20V5M6 11l6-6 6 6" /></Icon>;
export const ArrowIcon = () => <Icon><path d="M4 12h15M13 6l6 6-6 6" /></Icon>;
/** Two sheets, one over the other. */
export const CopyIcon = () => <Icon><rect x="8.5" y="8.5" width="12" height="12" /><path d="M15.5 8.5v-5h-12v12h5" /></Icon>;
/** A pencil. */
export const PenIcon = () => <Icon><path d="M4 20l1-4.5L15.5 5l3.5 3.5L8.5 19z" /><path d="M13 7.5l3.5 3.5" /></Icon>;
/** A tray with an arrow leaving it. */
export const ExportIcon = () => <Icon><path d="M12 15V3.5M7.5 8L12 3.5 16.5 8" /><path d="M4 13.5v7h16v-7" /></Icon>;
/** A tray with an arrow coming in. */
export const ImportIcon = () => <Icon><path d="M12 3.5V15M7.5 10.5L12 15l4.5-4.5" /><path d="M4 13.5v7h16v-7" /></Icon>;
/** A painter's palette. */
export const PaletteIcon = () => <Icon><path d="M12 3.5a8.5 8.5 0 1 0 0 17c1.4 0 1.9-.9 1.6-2-.4-1.4.3-2.5 1.9-2.5h2.2c1.6 0 2.8-1.2 2.8-2.8C20.5 7.5 16.7 3.5 12 3.5z" /><circle cx="7.8" cy="11" r="1" /><circle cx="10.5" cy="7.5" r="1" /><circle cx="15" cy="7.8" r="1" /></Icon>;
/** A plus. */
export const PlusIcon = () => <Icon><path d="M12 5v14M5 12h14" /></Icon>;
/** An open book. */
export const BookIcon = () => <Icon><path d="M12 6.5C10.3 5.2 7.6 4.5 3.5 4.5v14c4.1 0 6.8.7 8.5 2 1.7-1.3 4.4-2 8.5-2v-14c-4.1 0-6.8.7-8.5 2z" /><path d="M12 6.5v14" /></Icon>;
export const ChevronIcon = () => <Icon><path d="M6 9l6 6 6-6" /></Icon>;
export const GripIcon = () => <Icon><path d="M5 8h14M5 12h14M5 16h14" /></Icon>;
export const CloseIcon = () => <Icon><path d="M6 6l12 12M18 6L6 18" /></Icon>;
export const UpdateIcon = () => <Icon><path d="M20 11a8 8 0 0 0-14.3-4.3L3.5 9M3.5 4v5h5M4 13a8 8 0 0 0 14.3 4.3L20.5 15M20.5 20v-5h-5" /></Icon>;
/** A clock face with an arrow round it: the Versions tab of a mod. */
export const HistoryIcon = () => <Icon><path d="M3.5 12a8.5 8.5 0 1 0 2.5-6L3.5 8.5M3.5 4v4.5H8M12 7.5V12l3 2" /></Icon>;
export const ResetIcon = () => <Icon><path d="M4 4v6h6M4.5 15a8 8 0 1 0 1.5-8.5L4 10" /></Icon>;
export const InfoIcon = () => <Icon><circle cx="12" cy="12" r="8.5" /><path d="M12 11v5.5M12 7.6v.1" strokeWidth={2.4} /></Icon>;
/** Three sliders. */
export const SlidersIcon = () => <Icon><path d="M4 7h9M17 7h3M4 12h3M11 12h9M4 17h7M15 17h5" /><circle cx="15" cy="7" r="2" /><circle cx="9" cy="12" r="2" /><circle cx="13" cy="17" r="2" /></Icon>;
/** A keyboard with a mouse beside it: the Keys tab's mouse and keyboard column. */
export const KeyboardMouseIcon = () => <Icon><rect x="1.5" y="7.5" width="14" height="10" rx="1.3" /><path d="M4.6 10.8h.01M8.5 10.8h.01M12.4 10.8h.01M6 14.4h5" /><rect x="17.5" y="7.5" width="5" height="10" rx="2.5" /><path d="M20 7.5v3.2" /></Icon>;
/** A controller: the Keys tab's controller column. */
export const GamepadIcon = () => <Icon><path d="M7.2 7h9.6a4.4 4.4 0 0 1 4.3 3.5l1 5.2a2.5 2.5 0 0 1-4.4 2.1L15.5 15.5h-7l-2.2 2.3a2.5 2.5 0 0 1-4.4-2.1l1-5.2A4.4 4.4 0 0 1 7.2 7z" /><path d="M7.8 9.8v3.4M6.1 11.5h3.4M15.6 10.6h.01M17.6 12.6h.01" /></Icon>;
export const KeyboardIcon = () => <Icon><rect x="2.5" y="6" width="19" height="12" rx="1.5" /><path d="M6.5 10h.01M10 10h.01M14 10h.01M17.5 10h.01M6.5 14h.01M17.5 14h.01M10 14h4" /></Icon>;
/** A sheet with a folded corner. */
export const FileIcon = () => <Icon><path d="M6 3.5h8l4.5 4.5v12.5H6z" /><path d="M13.5 3.5v5h5" /></Icon>;
/** Two chain links. */
export const LinkIcon = () => <Icon><path d="M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1" /><path d="M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1" /></Icon>;
export const BellIcon = () => <Icon><path d="M6.4 15.6V9.8a5.6 5.6 0 0 1 11.2 0v5.8l1.6 2H4.8z" /><path d="M10.2 20.2h3.6" /></Icon>;
export const ExternalIcon = () => <Icon width="14" height="14"><path d="M10 5H5v14h14v-5" /><path d="M14 4h6v6M20 4l-9 9" /></Icon>;
export const DiscordIcon = () => (
  <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor" aria-hidden="true"><path d="M20.317 4.37a19.79 19.79 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.21.375-.444.865-.608 1.25a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.25.077.077 0 0 0-.079-.037A19.74 19.74 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.058a.082.082 0 0 0 .031.056 19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028c.462-.63.874-1.295 1.226-1.994a.076.076 0 0 0-.041-.106 13.1 13.1 0 0 1-1.872-.892.077.077 0 0 1-.008-.128c.126-.094.252-.192.372-.291a.074.074 0 0 1 .078-.01c3.928 1.793 8.18 1.793 12.062 0a.074.074 0 0 1 .078.009c.12.099.246.198.373.292a.077.077 0 0 1-.006.127 12.3 12.3 0 0 1-1.873.892.077.077 0 0 0-.041.107c.36.698.772 1.363 1.225 1.993a.076.076 0 0 0 .084.028 19.84 19.84 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.03zM8.02 15.33c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.956 2.418-2.157 2.418zm7.975 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.946 2.418-2.157 2.418z" /></svg>
);
export const KofiIcon = () => (
  <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor" aria-hidden="true"><path d="M11.351 2.715c-2.7 0-4.986.025-6.83.26C2.078 3.285 0 5.154 0 8.61c0 3.506.182 6.13 1.585 8.493 1.584 2.701 4.233 4.182 7.662 4.182h.83c4.209 0 6.494-2.234 7.637-4a9.5 9.5 0 0 0 1.091-2.338C21.792 14.688 24 12.22 24 9.208v-.415c0-3.247-2.13-5.507-5.792-5.87-1.558-.156-2.65-.208-6.857-.208m0 1.947c4.208 0 5.09.052 6.571.182 2.624.311 4.13 1.584 4.13 4v.39c0 2.156-1.792 3.844-3.87 3.844h-.935l-.156.649c-.208 1.013-.597 1.818-1.039 2.546-.909 1.428-2.545 3.064-5.922 3.064h-.805c-2.571 0-4.831-.883-6.078-3.195-1.09-2-1.298-4.155-1.298-7.506 0-2.181.857-3.402 3.012-3.714 1.533-.233 3.559-.26 6.39-.26m6.547 2.287c-.416 0-.65.234-.65.546v2.935c0 .311.234.545.65.545 1.324 0 2.051-.754 2.051-2s-.727-2.026-2.052-2.026m-10.39.182c-1.818 0-3.013 1.48-3.013 3.142 0 1.533.858 2.857 1.949 3.897.727.701 1.87 1.429 2.649 1.896a1.47 1.47 0 0 0 1.507 0c.78-.467 1.922-1.195 2.623-1.896 1.117-1.039 1.974-2.364 1.974-3.897 0-1.662-1.247-3.142-3.039-3.142-1.065 0-1.792.545-2.338 1.298-.493-.753-1.246-1.298-2.312-1.298" /></svg>
);

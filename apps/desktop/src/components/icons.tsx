import type { SVGProps, ReactNode } from "react";

type IconProps = SVGProps<SVGSVGElement> & {
  size?: number;
  sw?: number;
  d?: string;
  children?: ReactNode;
};

const I = ({
  d,
  size = 14,
  sw = 1.5,
  fill = "none",
  stroke = "currentColor",
  children,
  ...rest
}: IconProps) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill={fill}
    stroke={stroke}
    strokeWidth={sw}
    strokeLinecap="round"
    strokeLinejoin="round"
    style={{ flexShrink: 0, display: "block" }}
    {...rest}
  >
    {d ? <path d={d} /> : children}
  </svg>
);

export const Icon = {
  chevronRight: (p: IconProps) => <I {...p} d="M9 6l6 6-6 6" />,
  chevronDown: (p: IconProps) => <I {...p} d="M6 9l6 6 6-6" />,
  chevronLeft: (p: IconProps) => <I {...p} d="M15 6l-6 6 6 6" />,
  chevronsDown: (p: IconProps) => (
    <I {...p}>
      <path d="M6 6l6 6 6-6" />
      <path d="M6 13l6 6 6-6" />
    </I>
  ),

  database: (p: IconProps) => (
    <I {...p}>
      <ellipse cx="12" cy="5" rx="8" ry="2.5" />
      <path d="M4 5v6c0 1.4 3.6 2.5 8 2.5s8-1.1 8-2.5V5" />
      <path d="M4 11v6c0 1.4 3.6 2.5 8 2.5s8-1.1 8-2.5v-6" />
    </I>
  ),
  table: (p: IconProps) => (
    <I {...p}>
      <rect x="3" y="4" width="18" height="16" rx="1.5" />
      <path d="M3 10h18M3 15h18M10 4v16" />
    </I>
  ),
  view: (p: IconProps) => (
    <I {...p}>
      <rect x="3" y="4" width="18" height="16" rx="1.5" strokeDasharray="2 1.5" />
      <path d="M3 10h18M3 15h18M10 4v16" strokeDasharray="2 1.5" />
    </I>
  ),
  fn: (p: IconProps) => (
    <I {...p}>
      <path d="M16 4c-2 0-3 1-3 4v8c0 3-1 4-3 4" />
      <path d="M7 12h10" />
    </I>
  ),
  proc: (p: IconProps) => (
    <I {...p}>
      <path d="M4 7h16M4 12h16M4 17h10" />
      <circle cx="18" cy="17" r="2" />
    </I>
  ),
  schema: (p: IconProps) => (
    <I {...p} d="M3 6h7l2 2h9v11a1 1 0 01-1 1H3a1 1 0 01-1-1V7a1 1 0 011-1z" />
  ),

  plus: (p: IconProps) => <I {...p} d="M12 5v14M5 12h14" />,
  close: (p: IconProps) => <I {...p} d="M6 6l12 12M18 6L6 18" />,
  check: (p: IconProps) => <I {...p} d="M5 12l5 5L20 7" />,
  search: (p: IconProps) => (
    <I {...p}>
      <circle cx="11" cy="11" r="6" />
      <path d="M20 20l-4.5-4.5" />
    </I>
  ),
  more: (p: IconProps) => (
    <I {...p}>
      <circle cx="5" cy="12" r="1" fill="currentColor" />
      <circle cx="12" cy="12" r="1" fill="currentColor" />
      <circle cx="19" cy="12" r="1" fill="currentColor" />
    </I>
  ),
  settings: (p: IconProps) => (
    <I {...p}>
      <circle cx="12" cy="12" r="3" />
      <path d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 01-2.83 2.83l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 01-4 0v-.09A1.65 1.65 0 009 19.4a1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 01-2.83-2.83l.06-.06a1.65 1.65 0 00.33-1.82 1.65 1.65 0 00-1.51-1H3a2 2 0 010-4h.09A1.65 1.65 0 004.6 9a1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 012.83-2.83l.06.06a1.65 1.65 0 001.82.33H9a1.65 1.65 0 001-1.51V3a2 2 0 014 0v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 012.83 2.83l-.06.06a1.65 1.65 0 00-.33 1.82V9c.34.36.78.59 1.27.65H21a2 2 0 010 4h-.09a1.65 1.65 0 00-1.51 1z" />
    </I>
  ),
  sparkles: (p: IconProps) => (
    <I {...p}>
      <path d="M12 3l2 5 5 2-5 2-2 5-2-5-5-2 5-2z" />
      <path d="M19 13l1 2 2 1-2 1-1 2-1-2-2-1 2-1z" />
    </I>
  ),
  send: (p: IconProps) => <I {...p} d="M4 12l16-8-6 18-3-7z" />,
  paperclip: (p: IconProps) => (
    <I {...p} d="M21 11l-9 9a5.5 5.5 0 01-7.8-7.8l9-9a3.7 3.7 0 015.2 5.2l-9 9a1.8 1.8 0 01-2.6-2.6l8-8" />
  ),
  context: (p: IconProps) => (
    <I {...p}>
      <path d="M4 4h7v7H4zM13 4h7v7h-7zM4 13h7v7H4zM13 13h7v7h-7z" />
    </I>
  ),
  history: (p: IconProps) => (
    <I {...p}>
      <path d="M3 12a9 9 0 109-9 9 9 0 00-6.4 2.6L3 8" />
      <path d="M3 3v5h5M12 7v5l3 2" />
    </I>
  ),
  tree: (p: IconProps) => (
    <I {...p}>
      <circle cx="5" cy="5" r="2" />
      <circle cx="5" cy="19" r="2" />
      <circle cx="19" cy="12" r="2" />
      <path d="M5 7v10M7 5h6.5a3.5 3.5 0 013.5 3.5V12M7 19h6.5a3.5 3.5 0 003.5-3.5V12" />
    </I>
  ),
  terminal: (p: IconProps) => (
    <I {...p}>
      <path d="M5 8l4 4-4 4M13 16h6" />
      <rect x="2" y="4" width="20" height="16" rx="1.5" />
    </I>
  ),
  warn: (p: IconProps) => (
    <I {...p}>
      <path d="M12 9v4M12 17h.01" />
      <path d="M10.3 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.41 0z" />
    </I>
  ),
  info: (p: IconProps) => (
    <I {...p}>
      <circle cx="12" cy="12" r="9" />
      <path d="M12 16v-4M12 8h.01" />
    </I>
  ),
  eye: (p: IconProps) => (
    <I {...p}>
      <path d="M2 12s3.5-6 10-6 10 6 10 6-3.5 6-10 6S2 12 2 12z" />
      <circle cx="12" cy="12" r="2.5" />
    </I>
  ),
  eyeOff: (p: IconProps) => (
    <I {...p}>
      <path d="M3 3l18 18" />
      <path d="M10.6 6.2A10.2 10.2 0 0112 6c6.5 0 10 6 10 6a16.4 16.4 0 01-3.1 3.8" />
      <path d="M6.4 6.8A16.6 16.6 0 002 12s3.5 6 10 6a9.8 9.8 0 004.1-.9" />
      <path d="M10.2 10.2a2.5 2.5 0 003.5 3.5" />
    </I>
  ),
  user: (p: IconProps) => (
    <I {...p}>
      <circle cx="12" cy="8" r="4" />
      <path d="M4 21v-1a7 7 0 0114 0v1" />
    </I>
  ),
  lock: (p: IconProps) => (
    <I {...p}>
      <rect x="4" y="11" width="16" height="10" rx="2" />
      <path d="M8 11V7a4 4 0 018 0v4" />
    </I>
  ),
  power: (p: IconProps) => <I {...p} d="M12 2v10M18.4 6.6a9 9 0 11-12.8 0" />,
  splitH: (p: IconProps) => (
    <I {...p}>
      <rect x="3" y="3" width="18" height="18" rx="1.5" />
      <path d="M3 12h18" strokeDasharray="2 1.5" />
    </I>
  ),
  splitV: (p: IconProps) => (
    <I {...p}>
      <rect x="3" y="3" width="18" height="18" rx="1.5" />
      <path d="M12 3v18" strokeDasharray="2 1.5" />
    </I>
  ),
  panelLeft: (p: IconProps) => (
    <I {...p}>
      <rect x="3" y="3" width="18" height="18" rx="1.5" />
      <path d="M9 3v18" />
    </I>
  ),
  panelBottom: (p: IconProps) => (
    <I {...p}>
      <rect x="3" y="3" width="18" height="18" rx="1.5" />
      <path d="M3 15h18" />
    </I>
  ),
  panelRight: (p: IconProps) => (
    <I {...p}>
      <rect x="3" y="3" width="18" height="18" rx="1.5" />
      <path d="M15 3v18" />
    </I>
  ),
  expand: (p: IconProps) => <I {...p} d="M3 9V3h6M21 9V3h-6M3 15v6h6M21 15v6h-6" />,
  fileText: (p: IconProps) => (
    <I {...p}>
      <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" />
      <path d="M14 2v6h6M8 13h8M8 17h5" />
    </I>
  ),
  diff: (p: IconProps) => (
    <I {...p}>
      <path d="M12 3v18M9 6L6 9l3 3M15 18l3-3-3-3" />
    </I>
  ),
  download: (p: IconProps) => (
    <I {...p}>
      <path d="M12 3v12" />
      <path d="M7 10l5 5 5-5" />
      <path d="M5 21h14" />
    </I>
  ),
  upload: (p: IconProps) => (
    <I {...p}>
      <path d="M12 21V9" />
      <path d="M7 14l5-5 5 5" />
      <path d="M5 3h14" />
    </I>
  ),
  commit: (p: IconProps) => (
    <I {...p}>
      <circle cx="12" cy="12" r="4" />
      <path d="M16 12h6M2 12h6" />
    </I>
  ),
  copy: (p: IconProps) => (
    <I {...p}>
      <rect x="9" y="9" width="11" height="11" rx="1.5" />
      <path d="M5 15H4a1 1 0 01-1-1V4a1 1 0 011-1h10a1 1 0 011 1v1" />
    </I>
  ),
  edit: (p: IconProps) => (
    <I {...p} d="M11 4H5a2 2 0 00-2 2v12a2 2 0 002 2h12a2 2 0 002-2v-6M18 2l4 4-10 10H8v-4z" />
  ),
  undo: (p: IconProps) => (
    <I {...p} d="M3 8h10a5 5 0 010 10H8M3 8l4-4M3 8l4 4" />
  ),
  ssh: (p: IconProps) => (
    <I {...p}>
      <rect x="2" y="4" width="20" height="16" rx="1.5" />
      <path d="M6 9l3 3-3 3M13 15h5" />
    </I>
  ),
  cloud: (p: IconProps) => (
    <I
      {...p}
      d="M17.5 19a4.5 4.5 0 000-9c-.4-2.8-2.8-5-5.7-5a5.8 5.8 0 00-5.6 4.3A4.4 4.4 0 002 14a4 4 0 004 4z"
    />
  ),
  bolt: (p: IconProps) => (
    <I {...p} fill="currentColor" stroke="none" d="M13 2L4 14h6l-1 8 9-12h-6z" />
  ),
  play: (p: IconProps) => (
    <I {...p} fill="currentColor" stroke="none" d="M7 4.5v15l12-7.5z" />
  ),
  playSm: (p: IconProps) => (
    <I {...p} fill="currentColor" stroke="none" d="M8 5.5v13l10-6.5z" />
  ),
  format: (p: IconProps) => (
    <I {...p} d="M4 6h16M4 10h10M4 14h16M4 18h8" />
  ),
  wrap: (p: IconProps) => (
    <I {...p}>
      <path d="M4 6h16M4 18h6" />
      <path d="M4 12h14a3 3 0 010 6h-3" />
      <path d="M12 15l-2 3 2 3" />
    </I>
  ),
  star: (p: IconProps) => (
    <I {...p} d="M12 3l2.6 5.6 6 .7-4.4 4.1 1.2 6L12 16.9 6.6 19.4l1.2-6L3.4 9.3l6-.7z" />
  ),
  pin: (p: IconProps) => (
    <I {...p} d="M9 4h6l-1 6 3 3H7l3-3-1-6zM12 13v7" />
  ),
  layout: (p: IconProps) => (
    <I {...p}>
      <rect x="3" y="3" width="18" height="18" rx="1.5" />
      <path d="M3 9h18M9 9v12" />
    </I>
  ),
  bracket: (p: IconProps) => <I {...p} d="M8 4H4v16h4M16 4h4v16h-4" />,
  trash: (p: IconProps) => (
    <I {...p}>
      <path d="M3 6h18" />
      <path d="M8 6V4a1 1 0 011-1h6a1 1 0 011 1v2" />
      <path d="M19 6l-1.4 13.1a2 2 0 01-2 1.9H8.4a2 2 0 01-2-1.9L5 6" />
      <path d="M10 11v6M14 11v6" />
    </I>
  ),
};

export type IconName = keyof typeof Icon;

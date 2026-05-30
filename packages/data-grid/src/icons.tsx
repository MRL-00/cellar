import type { SVGProps, ReactNode } from "react";

/**
 * Tiny self-contained icon set used by the grid. Kept inside the data-grid
 * package so it has no dependency on the desktop app's icon registry — the
 * grid is meant to live in its own workspace and ship to plugins.
 */
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

export const GridIcon = {
  plus: (p: IconProps) => <I {...p} d="M12 5v14M5 12h14" />,
  close: (p: IconProps) => <I {...p} d="M6 6l12 12M18 6L6 18" />,
  undo: (p: IconProps) => <I {...p} d="M3 8h10a5 5 0 010 10H8M3 8l4-4M3 8l4 4" />,
  commit: (p: IconProps) => (
    <I {...p}>
      <circle cx="12" cy="12" r="4" />
      <path d="M16 12h6M2 12h6" />
    </I>
  ),
  diff: (p: IconProps) => (
    <I {...p}>
      <path d="M12 3v18M9 6L6 9l3 3M15 18l3-3-3-3" />
    </I>
  ),
  filter: (p: IconProps) => <I {...p} d="M3 5h18l-7 9v6l-4-2v-4z" />,
  sortAsc: (p: IconProps) => (
    <I {...p}>
      <path d="M7 4v16M3 16l4 4 4-4" />
    </I>
  ),
  sortDesc: (p: IconProps) => (
    <I {...p}>
      <path d="M7 20V4M3 8l4-4 4 4" />
    </I>
  ),
  key: (p: IconProps) => (
    <I {...p}>
      <circle cx="8" cy="15" r="4" />
      <path d="M11 12l8-8M16 7l3 3M14 9l3 3" />
    </I>
  ),
  link: (p: IconProps) => (
    <I {...p}>
      <path d="M10 14L4 20M14 10l6-6M10 6V3h11v11h-3M14 18v3H3V10h3" />
    </I>
  ),
  link2: (p: IconProps) => (
    <I {...p}>
      <path d="M9 17H7A5 5 0 017 7h2M15 7h2a5 5 0 010 10h-2M8 12h8" />
    </I>
  ),
  text: (p: IconProps) => <I {...p} d="M4 7V5h16v2M12 5v14M9 19h6" />,
  hash: (p: IconProps) => <I {...p} d="M4 9h16M4 15h16M10 3L8 21M16 3l-2 18" />,
  cal: (p: IconProps) => (
    <I {...p}>
      <rect x="3" y="5" width="18" height="16" rx="1.5" />
      <path d="M3 10h18M8 3v4M16 3v4" />
    </I>
  ),
  bool: (p: IconProps) => (
    <I {...p}>
      <rect x="2" y="6" width="20" height="12" rx="6" />
      <circle cx="8" cy="12" r="3" fill="currentColor" />
    </I>
  ),
  json: (p: IconProps) => (
    <I
      {...p}
      d="M7 4S4 4 4 8s2 4 2 4-2 0-2 4 3 4 3 4M17 4s3 0 3 4-2 4-2 4 2 0 2 4-3 4-3 4"
    />
  ),
  enum: (p: IconProps) => (
    <I {...p}>
      <circle cx="6" cy="6" r="2" />
      <circle cx="6" cy="12" r="2" />
      <circle cx="6" cy="18" r="2" />
      <path d="M11 6h10M11 12h10M11 18h10" />
    </I>
  ),
};

export type GridIconProps = IconProps;

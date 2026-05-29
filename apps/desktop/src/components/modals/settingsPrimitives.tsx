import type { ReactNode } from "react";
import { Icon } from "../icons";

export const ED_RUN_BASE =
  "inline-flex h-[26px] items-center gap-[5px] whitespace-nowrap rounded-[4px] border border-transparent px-2.5 text-[11.5px] font-medium text-fg-1 transition-[background,color,border-color,filter] duration-[120ms]";
export const ED_RUN_SUBTLE =
  ED_RUN_BASE +
  " bg-transparent border-border-default hover:bg-bg-3 hover:border-border-strong hover:text-fg-0";
export const ED_RUN_PRIMARY =
  ED_RUN_BASE + " bg-accent text-accent-fg hover:brightness-[1.07]";
export const ED_RUN_DANGER =
  ED_RUN_BASE +
  " bg-transparent border-[color-mix(in_oklab,var(--delete)_30%,var(--border-default))] text-delete hover:bg-delete-bg hover:border-delete";

export const CD_INPUT =
  "h-[26px] min-w-0 flex-1 rounded-[4px] border border-border-default bg-bg-inset px-2 text-[11.5px] text-fg-0 outline-none focus:border-accent-line focus:bg-bg-2";

export function Section({
  title,
  sub,
  children,
}: {
  title: string;
  sub?: string;
  children: ReactNode;
}) {
  return (
    <section className="px-6 pb-1 pt-[18px] [&+section]:mt-1.5 [&+section]:border-t [&+section]:border-border-divider">
      <header className="mb-3">
        <h2 className="m-0 text-[13px] font-semibold tracking-[-0.005em] text-fg-0">
          {title}
        </h2>
        {sub && (
          <p className="m-0 mt-px max-w-[60ch] text-[11.5px] text-fg-2 text-pretty">
            {sub}
          </p>
        )}
      </header>
      <div className="flex flex-col gap-2">{children}</div>
    </section>
  );
}

export function Row({
  label,
  hint,
  children,
  stack,
}: {
  label: string;
  hint?: string;
  children: ReactNode;
  stack?: boolean;
}) {
  return (
    <div
      className={
        "grid min-h-[24px] grid-cols-[180px_1fr] gap-[14px] py-1 " +
        (stack ? "items-start" : "items-center")
      }
    >
      <div className="flex flex-col gap-[2px] text-[11.5px] font-medium text-fg-1">
        <span>{label}</span>
        {hint && (
          <span className="text-[10.5px] font-normal text-fg-3 text-pretty">
            {hint}
          </span>
        )}
      </div>
      <div
        className={
          "min-w-0 text-[11.5px] " +
          (stack ? "block" : "flex flex-wrap items-center gap-2")
        }
      >
        {children}
      </div>
    </div>
  );
}

export function Toggle({
  on,
  onChange,
  locked,
  ariaLabel = "Setting toggle",
}: {
  on: boolean;
  onChange?: (v: boolean) => void;
  locked?: boolean;
  ariaLabel?: string;
}) {
  const interactive = Boolean(onChange) && !locked;
  return (
    <button
      type="button"
      role="switch"
      aria-checked={on}
      aria-label={ariaLabel}
      onClick={() => interactive && onChange?.(!on)}
      disabled={!interactive}
      title={locked ? "Forced on for prod connections" : undefined}
      className={
        "relative h-4 w-7 shrink-0 rounded-[10px] transition-colors duration-150 " +
        (on ? (locked ? "bg-warn" : "bg-accent") : "bg-bg-3") +
        (!interactive ? " cursor-not-allowed opacity-85" : "")
      }
    >
      <span
        className={
          "absolute top-[2px] h-3 w-3 rounded-full bg-white transition-[left] duration-150 " +
          (on ? "left-[14px]" : "left-[2px]")
        }
      />
    </button>
  );
}

export function Segment<T extends string>({
  options,
  value,
  onChange,
}: {
  options: { value: T; label: string }[];
  value: T;
  onChange?: (v: T) => void;
}) {
  return (
    <div className="inline-flex gap-px rounded-[4px] border border-border-default bg-bg-inset p-[2px]">
      {options.map((o) => (
        <button
          type="button"
          key={o.value}
          disabled={!onChange}
          aria-pressed={value === o.value}
          onClick={() => onChange?.(o.value)}
          className={
            "h-5 rounded-[3px] px-2.5 text-[11px] " +
            (value === o.value
              ? "bg-bg-3 font-medium text-fg-0"
              : "text-fg-2 " + (onChange ? "hover:text-fg-0" : ""))
          }
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}

export function StaticSegment({
  values,
  activeIdx,
}: {
  values: string[];
  activeIdx: number;
}) {
  return (
    <div className="inline-flex gap-px rounded-[4px] border border-border-default bg-bg-inset p-[2px]">
      {values.map((v, i) => (
        <button
          type="button"
          key={v}
          disabled
          aria-pressed={i === activeIdx}
          className={
            "h-5 cursor-not-allowed rounded-[3px] px-2.5 text-[11px] " +
            (i === activeIdx
              ? "bg-bg-3 font-medium text-fg-0"
              : "text-fg-2")
          }
        >
          {v}
        </button>
      ))}
    </div>
  );
}

export function StubBanner({ children }: { children: ReactNode }) {
  return (
    <div className="mx-6 my-2 flex items-center gap-1.5 rounded-[4px] border border-dashed border-border-default bg-bg-inset px-3 py-2 text-[11px] text-fg-2">
      <Icon.info size={12} stroke="var(--fg-3)" />
      <span>{children}</span>
    </div>
  );
}

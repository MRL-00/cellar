import { useEffect, useRef } from "react";

import {
  isFilled,
  PARAM_INPUT_LABELS,
  PARAM_INPUT_TYPES,
  toCellValue,
  type ParamInputType,
} from "../lib/queryParamValues";
import { useQueryParams } from "../state/queryParams";
import { Icon } from "./icons";

interface ParameterPanelProps {
  tabId: string;
  running: boolean;
  /** Validate the current values and run, or focus the first empty input. */
  onRun: () => void;
  onClose: () => void;
  /** Called when any value/type changes, so a stale run-error marker clears. */
  onEdit?: () => void;
}

/**
 * Inline panel shown below the editor when the statement being run contains
 * named (`:name`) or positional (`$N`) parameters. Collects a typed value per
 * parameter before binding them server-side.
 */
export function ParameterPanel({
  tabId,
  running,
  onRun,
  onClose,
  onEdit,
}: ParameterPanelProps) {
  const panel = useQueryParams((s) => s.panels[tabId]);
  const setStoreValue = useQueryParams((s) => s.setValue);
  const inputs = useRef<Record<string, HTMLElement | null>>({});

  // Editing a value invalidates the previous run's error marker, just like
  // editing the SQL buffer does.
  const setValue = (name: string, patch: Parameters<typeof setStoreValue>[2]) => {
    onEdit?.();
    setStoreValue(tabId, name, patch);
  };

  const focusRequest = panel?.focusRequest ?? 0;
  useEffect(() => {
    if (!panel) return;
    // Target the first field that blocks a run: empty, or non-empty but failing
    // validation (e.g. a bad number/date), so run-blocked focus lands on the
    // field that actually needs fixing.
    const firstProblem = panel.params.find((p) => {
      const value = panel.values[p.name];
      return !value || !isFilled(value) || !toCellValue(value).ok;
    });
    const target = firstProblem
      ? inputs.current[firstProblem.name]
      : inputs.current[panel.params[0]?.name ?? ""];
    target?.focus();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [focusRequest]);

  if (!panel) return null;

  const onKeyDown = (e: React.KeyboardEvent) => {
    if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
      e.preventDefault();
      e.stopPropagation();
      if (!running) onRun();
    }
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    }
  };

  return (
    <div className="param-panel mono" onKeyDown={onKeyDown}>
      <div className="param-panel-head">
        <span className="param-panel-title">
          <Icon.sparkles size={11} style={{ color: "var(--accent)" }} />
          Parameters
        </span>
        <span className="param-panel-hint">
          fill values, then <span className="kbd">⌘⏎</span> to run
        </span>
        <button
          className="icon-btn"
          onClick={onClose}
          title="Dismiss parameters"
          aria-label="Dismiss parameters"
        >
          <Icon.close size={12} />
        </button>
      </div>

      <div className="param-panel-grid">
        {panel.params.map((param) => {
          const value = panel.values[param.name] ?? { type: "text", value: "" };
          const conversion = toCellValue(value);
          const invalid = !conversion.ok;
          return (
            <div className="param-row" key={param.name}>
              <label className="param-label" htmlFor={`param-${tabId}-${param.name}`}>
                <span className="param-placeholder">{param.placeholder}</span>
                {param.column_hint && (
                  <span className="param-col-hint">→ {param.column_hint}</span>
                )}
              </label>
              <select
                className="param-type"
                value={value.type}
                onChange={(e) =>
                  setValue(param.name, {
                    type: e.target.value as ParamInputType,
                    value:
                      e.target.value === "boolean" ? "false" : value.value,
                  })
                }
                aria-label={`Type for ${param.placeholder}`}
              >
                {PARAM_INPUT_TYPES.map((t) => (
                  <option key={t} value={t}>
                    {PARAM_INPUT_LABELS[t]}
                  </option>
                ))}
              </select>
              <ValueInput
                id={`param-${tabId}-${param.name}`}
                type={value.type}
                value={value.value}
                invalid={invalid}
                registerRef={(el) => {
                  inputs.current[param.name] = el;
                }}
                onChange={(v) => setValue(param.name, { value: v })}
              />
              {invalid && !conversion.ok && (
                <span className="param-error">{conversion.error}</span>
              )}
            </div>
          );
        })}
      </div>

      <div className="param-panel-foot">
        <button
          className="ed-run primary"
          onClick={onRun}
          disabled={running}
          title="Run with these parameter values"
        >
          <Icon.playSm size={11} />
          <span>{running ? "Running…" : "Run"}</span>
          <span className="kbd">⌘⏎</span>
        </button>
      </div>
    </div>
  );
}

interface ValueInputProps {
  id: string;
  type: ParamInputType;
  value: string;
  invalid: boolean;
  registerRef: (el: HTMLElement | null) => void;
  onChange: (value: string) => void;
}

function ValueInput({
  id,
  type,
  value,
  invalid,
  registerRef,
  onChange,
}: ValueInputProps) {
  if (type === "null") {
    return (
      <span className="param-null" ref={registerRef as (el: HTMLSpanElement | null) => void}>
        NULL
      </span>
    );
  }
  if (type === "boolean") {
    return (
      <select
        id={id}
        ref={registerRef as (el: HTMLSelectElement | null) => void}
        className="param-value"
        value={value === "true" ? "true" : "false"}
        onChange={(e) => onChange(e.target.value)}
      >
        <option value="true">true</option>
        <option value="false">false</option>
      </select>
    );
  }
  return (
    <input
      id={id}
      ref={registerRef as (el: HTMLInputElement | null) => void}
      className={"param-value" + (invalid ? " invalid" : "")}
      type={type === "number" ? "text" : "text"}
      inputMode={type === "number" ? "decimal" : undefined}
      placeholder={type === "date" ? "YYYY-MM-DD" : ""}
      value={value}
      spellCheck={false}
      autoComplete="off"
      onChange={(e) => onChange(e.target.value)}
    />
  );
}

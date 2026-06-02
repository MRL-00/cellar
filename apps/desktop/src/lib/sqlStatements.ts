// Statement-aware slicing of a SQL buffer. The editor uses this to find the
// statement under the cursor (for ⌘⏎ "Run current statement") and to paint the
// in-statement highlight band. Splitting happens at top-level `;` only — a
// semicolon inside a string literal, comment, or dollar-quoted block is not a
// statement boundary, so this scanner tracks those contexts explicitly rather
// than doing a naive `split(";")`.

export interface SqlStatement {
  /** Trimmed statement text, excluding the terminating semicolon. */
  text: string;
  /** Offset of the first character of `text` in the source buffer. */
  start: number;
  /** Offset just past the last character of `text` (before any `;`). */
  end: number;
  /** Start of the contiguous source chunk this statement owns. */
  rawStart: number;
  /** End (exclusive) of the contiguous source chunk, including its `;`. */
  rawEnd: number;
  /** 1-based line of `start`. */
  startLine: number;
  /** 1-based line of the statement's last content character. */
  endLine: number;
}

const isWs = (c: string) => c === " " || c === "\t" || c === "\n" || c === "\r";

/** Skip a `'…'` or `"…"` literal, honouring doubled-quote and backslash escapes. */
function skipQuoted(sql: string, i: number, quote: string): number {
  let j = i + 1;
  while (j < sql.length) {
    const c = sql[j];
    if (c === "\\") {
      j += 2;
      continue;
    }
    if (c === quote) {
      if (sql[j + 1] === quote) {
        j += 2;
        continue;
      }
      return j + 1;
    }
    j++;
  }
  return sql.length;
}

/** Skip a `-- …` line comment up to (but not including) the newline. */
function skipLineComment(sql: string, i: number): number {
  let j = i + 2;
  while (j < sql.length && sql[j] !== "\n") j++;
  return j;
}

/** Skip a `/* … *\/` block comment, including the closing delimiter. */
function skipBlockComment(sql: string, i: number): number {
  let j = i + 2;
  while (j < sql.length && !(sql[j] === "*" && sql[j + 1] === "/")) j++;
  return Math.min(sql.length, j + 2);
}

/**
 * If a dollar-quote tag (`$$` or `$tag$`) opens at `i`, return the offset just
 * past its matching close. Otherwise return -1 so the caller treats `$` as an
 * ordinary character.
 */
function skipDollarQuote(sql: string, i: number): number {
  const open = /^\$[A-Za-z_]?[A-Za-z0-9_]*\$/.exec(sql.slice(i));
  if (!open) return -1;
  const tag = open[0];
  const close = sql.indexOf(tag, i + tag.length);
  return close < 0 ? sql.length : close + tag.length;
}

/** Offsets of every top-level (statement-terminating) semicolon. */
function topLevelSemicolons(sql: string): number[] {
  const cuts: number[] = [];
  let i = 0;
  while (i < sql.length) {
    const c = sql[i];
    if (c === "'" || c === '"') {
      i = skipQuoted(sql, i, c);
      continue;
    }
    if (c === "-" && sql[i + 1] === "-") {
      i = skipLineComment(sql, i);
      continue;
    }
    if (c === "/" && sql[i + 1] === "*") {
      i = skipBlockComment(sql, i);
      continue;
    }
    if (c === "$") {
      const next = skipDollarQuote(sql, i);
      if (next >= 0) {
        i = next;
        continue;
      }
    }
    if (c === ";") {
      cuts.push(i);
    }
    i++;
  }
  return cuts;
}

function lineStarts(sql: string): number[] {
  const starts = [0];
  for (let i = 0; i < sql.length; i++) {
    if (sql[i] === "\n") starts.push(i + 1);
  }
  return starts;
}

/** 1-based line number for a source offset, via binary search over line starts. */
function lineAt(starts: number[], offset: number): number {
  let lo = 0;
  let hi = starts.length - 1;
  while (lo < hi) {
    const mid = (lo + hi + 1) >> 1;
    if ((starts[mid] as number) <= offset) lo = mid;
    else hi = mid - 1;
  }
  return lo + 1;
}

function makeStatement(
  sql: string,
  starts: number[],
  rawStart: number,
  rawEnd: number,
): SqlStatement {
  let s = rawStart;
  let e = rawEnd;
  if (e > s && sql[e - 1] === ";") e--;
  while (s < e && isWs(sql[s] as string)) s++;
  while (e > s && isWs(sql[e - 1] as string)) e--;
  return {
    text: sql.slice(s, e),
    start: s,
    end: e,
    rawStart,
    rawEnd,
    startLine: lineAt(starts, s),
    endLine: lineAt(starts, Math.max(s, e - 1)),
  };
}

/**
 * Partition the buffer into contiguous statement chunks covering `[0, len)`.
 * Whitespace-only chunks are kept (with `text === ""`) so offset lookups stay
 * total; callers filter them out when they need runnable statements.
 */
export function partitionStatements(sql: string): SqlStatement[] {
  const starts = lineStarts(sql);
  const cuts = topLevelSemicolons(sql);
  const out: SqlStatement[] = [];
  let prev = 0;
  for (const cut of cuts) {
    out.push(makeStatement(sql, starts, prev, cut + 1));
    prev = cut + 1;
  }
  if (prev < sql.length || out.length === 0) {
    out.push(makeStatement(sql, starts, prev, sql.length));
  }
  return out;
}

/** Non-empty statements only — the ones that can actually be executed. */
export function splitStatements(sql: string): SqlStatement[] {
  return partitionStatements(sql).filter((s) => s.text.length > 0);
}

/**
 * The statement under the caret. If the caret sits in blank space between
 * statements, fall back to the nearest preceding non-empty statement (matching
 * how most SQL editors resolve "run statement under cursor").
 */
export function statementAtOffset(
  sql: string,
  offset: number,
): SqlStatement | null {
  const chunks = partitionStatements(sql);
  const clamped = Math.max(0, Math.min(offset, sql.length));
  const containing = chunks.find(
    (c) => clamped >= c.rawStart && clamped < c.rawEnd,
  );
  if (containing && containing.text.length > 0) return containing;

  const nonEmpty = chunks.filter((c) => c.text.length > 0);
  if (nonEmpty.length === 0) return null;

  let preceding: SqlStatement | null = null;
  for (const c of nonEmpty) {
    if (c.rawStart <= clamped) preceding = c;
    else break;
  }
  return preceding ?? nonEmpty[0] ?? null;
}

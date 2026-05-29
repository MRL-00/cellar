import { Fragment, type ReactNode } from "react";

export type Token = { kind: TokKind; text: string };
export type TokKind =
  | "kw"
  | "fn"
  | "str"
  | "num"
  | "comment"
  | "op"
  | "tbl"
  | "ident"
  | "ws"
  | "nl";

const KEYWORDS = new Set([
  "select", "from", "where", "and", "or", "not", "in", "is", "null", "as",
  "join", "left", "right", "inner", "outer", "on", "group", "by", "order",
  "having", "limit", "offset", "with", "case", "when", "then", "else", "end",
  "insert", "into", "values", "update", "set", "delete", "begin", "commit",
  "rollback", "create", "alter", "drop", "table", "view", "index", "if",
  "exists", "filter", "distinct", "interval", "now", "between", "like",
  "ilike", "nulls", "first", "last", "asc", "desc", "returning", "default",
]);

const FUNCTIONS = new Set([
  "count", "sum", "avg", "min", "max", "round", "nullif", "coalesce",
  "date_trunc", "now", "concat", "length", "lower", "upper",
]);

const isSpace = (c: string) => c === " " || c === "\t";
const isDigit = (c: string) => c >= "0" && c <= "9";
const isAlpha = (c: string) =>
  (c >= "a" && c <= "z") || (c >= "A" && c <= "Z") || c === "_";
const isAlphaNum = (c: string) => isAlpha(c) || isDigit(c);

/** Tokenize a single SQL string, preserving whitespace and newlines. */
export function tokenizeSql(sql: string): Token[] {
  const out: Token[] = [];
  let i = 0;
  while (i < sql.length) {
    const ch = sql.charAt(i);
    if (ch === "\n") {
      out.push({ kind: "nl", text: "\n" });
      i++;
      continue;
    }
    if (isSpace(ch)) {
      let j = i;
      while (j < sql.length && isSpace(sql.charAt(j))) j++;
      out.push({ kind: "ws", text: sql.slice(i, j) });
      i = j;
      continue;
    }
    if (ch === "-" && sql.charAt(i + 1) === "-") {
      let j = i;
      while (j < sql.length && sql.charAt(j) !== "\n") j++;
      out.push({ kind: "comment", text: sql.slice(i, j) });
      i = j;
      continue;
    }
    if (ch === "'") {
      let j = i + 1;
      while (j < sql.length) {
        if (sql.charAt(j) !== "'") {
          j++;
          continue;
        }
        if (sql.charAt(j + 1) === "'") {
          j += 2;
          continue;
        }
        j++;
        break;
      }
      out.push({ kind: "str", text: sql.slice(i, j) });
      i = j;
      continue;
    }
    if (isDigit(ch)) {
      let j = i;
      while (j < sql.length) {
        const c = sql.charAt(j);
        if (!(isDigit(c) || c === ".")) break;
        j++;
      }
      out.push({ kind: "num", text: sql.slice(i, j) });
      i = j;
      continue;
    }
    if (isAlpha(ch)) {
      let j = i;
      while (j < sql.length && isAlphaNum(sql.charAt(j))) j++;
      const word = sql.slice(i, j);
      const lower = word.toLowerCase();
      let kind: TokKind = "ident";
      if (KEYWORDS.has(lower)) kind = "kw";
      else if (FUNCTIONS.has(lower) && sql.charAt(j) === "(") kind = "fn";
      else if (i > 0 && sql.charAt(i - 1) === ".") kind = "tbl";
      out.push({ kind, text: word });
      i = j;
      continue;
    }
    out.push({ kind: "op", text: ch });
    i++;
  }
  return out;
}

/** Split tokens into one array per source line. */
export function tokensToLines(tokens: Token[]): Token[][] {
  const lines: Token[][] = [[]];
  for (const t of tokens) {
    if (t.kind === "nl") lines.push([]);
    else (lines[lines.length - 1] as Token[]).push(t);
  }
  return lines;
}

const TOKEN_CLASS: Record<Exclude<TokKind, "ws" | "nl">, string> = {
  kw: "text-syn-kw",
  fn: "text-syn-fn",
  str: "text-syn-str",
  num: "text-syn-num",
  comment: "italic text-syn-comment",
  op: "text-syn-op",
  tbl: "text-syn-tbl",
  ident: "text-syn-ident",
};

export function renderTokens(tokens: Token[]): ReactNode {
  return tokens.map((t, i) => {
    if (t.kind === "ws") return t.text;
    if (t.kind === "nl") return null;
    return (
      <Fragment key={i}>
        <span className={TOKEN_CLASS[t.kind]}>{t.text}</span>
      </Fragment>
    );
  });
}

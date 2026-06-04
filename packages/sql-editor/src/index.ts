import {
  acceptCompletion,
  autocompletion,
  closeBrackets,
  completionKeymap,
  type Completion,
} from "@codemirror/autocomplete";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import {
  bracketMatching,
  HighlightStyle,
  indentOnInput,
  indentUnit,
  syntaxHighlighting,
} from "@codemirror/language";
import {
  MSSQL,
  MySQL,
  PostgreSQL,
  SQLite,
  StandardSQL,
  sql,
  type SQLDialect,
  type SQLNamespace,
} from "@codemirror/lang-sql";
import {
  Compartment,
  EditorState,
  StateField,
  type Extension,
  RangeSetBuilder,
  type Text,
} from "@codemirror/state";
import {
  Decoration,
  type DecorationSet,
  drawSelection,
  dropCursor,
  EditorView,
  GutterMarker,
  gutterLineClass,
  highlightActiveLine,
  highlightActiveLineGutter,
  keymap,
  lineNumbers,
  placeholder,
  rectangularSelection,
  type KeyBinding,
} from "@codemirror/view";
import { tags } from "@lezer/highlight";
import {
  useEffect,
  useMemo,
  useRef,
  createElement,
  type MutableRefObject,
} from "react";

export type SqlEditorDialect =
  | "postgres"
  | "mysql"
  | "sqlite"
  | "mssql"
  | "azure";

export interface SqlEditorStatementRange {
  startLine: number;
  endLine: number;
}

export interface SqlEditorCompletionColumn {
  name: string;
  data_type?: string;
  nullable?: boolean;
}

export interface SqlEditorCompletionRelation {
  name: string;
  schema: string;
  columns: SqlEditorCompletionColumn[];
}

export interface SqlEditorCompletionSchema {
  name: string;
  tables: SqlEditorCompletionRelation[];
  views: SqlEditorCompletionRelation[];
}

export interface SqlEditorCompletionDatabase {
  name: string;
  is_default?: boolean;
  schemas: SqlEditorCompletionSchema[];
}

export interface SqlEditorCompletionCache {
  databases: SqlEditorCompletionDatabase[];
  database?: string | null;
  defaultSchema?: string | null;
}

export interface CodeMirrorSqlEditorProps {
  value: string;
  dialect?: SqlEditorDialect;
  completionSchema?: SqlEditorCompletionCache;
  wrap?: boolean;
  placeholder?: string;
  statementRange?: SqlEditorStatementRange | null;
  errorLine?: number | null;
  className?: string;
  ariaLabel?: string;
  onChange: (value: string) => void;
  onCursorChange?: (offset: number) => void;
  onRunStatement?: () => void;
  onRunAll?: () => void;
}

interface HighlightConfig {
  statementRange?: SqlEditorStatementRange | null;
  errorLine?: number | null;
}

const dialects: Record<SqlEditorDialect, SQLDialect> = {
  postgres: PostgreSQL,
  mysql: MySQL,
  sqlite: SQLite,
  mssql: MSSQL,
  azure: MSSQL,
};

const statementLine = Decoration.line({
  class: "cm-cellar-current-statement",
});
const errorLine = Decoration.line({ class: "cm-cellar-error-line" });

class LineClassMarker extends GutterMarker {
  constructor(override readonly elementClass: string) {
    super();
  }

  override eq(other: GutterMarker): boolean {
    return (
      other instanceof LineClassMarker &&
      other.elementClass === this.elementClass
    );
  }
}

const statementGutter = new LineClassMarker("cm-cellar-current-statement-gutter");
const errorGutter = new LineClassMarker("cm-cellar-error-line-gutter");

export function CodeMirrorSqlEditor({
  value,
  dialect = "postgres",
  completionSchema,
  wrap = false,
  placeholder: placeholderText,
  statementRange,
  errorLine: errorLineNumber,
  className,
  ariaLabel = "SQL editor",
  onChange,
  onCursorChange,
  onRunStatement,
  onRunAll,
}: CodeMirrorSqlEditorProps) {
  const hostRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const valueRef = useRef(value);
  const onChangeRef = useLatest(onChange);
  const onCursorChangeRef = useLatest(onCursorChange);
  const onRunStatementRef = useLatest(onRunStatement);
  const onRunAllRef = useLatest(onRunAll);
  const compartments = useMemo(
    () => ({
      highlight: new Compartment(),
      language: new Compartment(),
      placeholder: new Compartment(),
      wrap: new Compartment(),
    }),
    [],
  );

  useEffect(() => {
    const host = hostRef.current;
    if (!host) return;

    const view = new EditorView({
      parent: host,
      state: EditorState.create({
        doc: valueRef.current,
        extensions: [
          cellarEditorTheme,
          syntaxHighlighting(cellarHighlightStyle),
          lineNumbers(),
          highlightActiveLineGutter(),
          highlightActiveLine(),
          drawSelection(),
          dropCursor(),
          rectangularSelection(),
          history(),
          bracketMatching(),
          closeBrackets(),
          autocompletion(),
          indentOnInput(),
          indentUnit.of("  "),
          EditorState.tabSize.of(2),
          EditorView.contentAttributes.of({ "aria-label": ariaLabel }),
          EditorView.updateListener.of((update) => {
            if (update.docChanged) {
              const next = update.state.doc.toString();
              valueRef.current = next;
              onChangeRef.current(next);
            }
            if (update.docChanged || update.selectionSet) {
              onCursorChangeRef.current?.(update.state.selection.main.head);
            }
          }),
          keymap.of(createKeymap(onRunStatementRef, onRunAllRef)),
          compartments.language.of(languageExtension(dialect, completionSchema)),
          compartments.wrap.of(wrap ? EditorView.lineWrapping : []),
          compartments.placeholder.of(
            placeholderText ? placeholder(placeholderText) : [],
          ),
          compartments.highlight.of(
            lineHighlightExtension({
              statementRange,
              errorLine: errorLineNumber,
            }),
          ),
        ],
      }),
    });

    viewRef.current = view;
    onCursorChangeRef.current?.(view.state.selection.main.head);

    return () => {
      view.destroy();
      viewRef.current = null;
    };
  }, []);

  useEffect(() => {
    const view = viewRef.current;
    if (!view) {
      valueRef.current = value;
      return;
    }
    if (value === valueRef.current && value === view.state.doc.toString()) {
      return;
    }

    valueRef.current = value;
    view.dispatch({
      changes: { from: 0, to: view.state.doc.length, insert: value },
    });
  }, [value]);

  useEffect(() => {
    viewRef.current?.dispatch({
      effects: compartments.language.reconfigure(
        languageExtension(dialect, completionSchema),
      ),
    });
  }, [compartments.language, completionSchema, dialect]);

  useEffect(() => {
    viewRef.current?.dispatch({
      effects: compartments.wrap.reconfigure(
        wrap ? EditorView.lineWrapping : [],
      ),
    });
  }, [compartments.wrap, wrap]);

  useEffect(() => {
    viewRef.current?.dispatch({
      effects: compartments.placeholder.reconfigure(
        placeholderText ? placeholder(placeholderText) : [],
      ),
    });
  }, [compartments.placeholder, placeholderText]);

  useEffect(() => {
    viewRef.current?.dispatch({
      effects: compartments.highlight.reconfigure(
        lineHighlightExtension({
          statementRange,
          errorLine: errorLineNumber,
        }),
      ),
    });
  }, [compartments.highlight, errorLineNumber, statementRange]);

  return createElement("div", { ref: hostRef, className });
}

function useLatest<T>(value: T): MutableRefObject<T> {
  const ref = useRef(value);
  useEffect(() => {
    ref.current = value;
  }, [value]);
  return ref;
}

function createKeymap(
  onRunStatementRef: MutableRefObject<(() => void) | undefined>,
  onRunAllRef: MutableRefObject<(() => void) | undefined>,
): KeyBinding[] {
  return [
    {
      key: "Mod-Enter",
      run() {
        onRunStatementRef.current?.();
        return true;
      },
    },
    {
      key: "Mod-Shift-Enter",
      run() {
        onRunAllRef.current?.();
        return true;
      },
    },
    {
      key: "Tab",
      run(view) {
        if (acceptCompletion(view)) return true;
        view.dispatch(view.state.replaceSelection("  "));
        return true;
      },
    },
    ...completionKeymap,
    ...historyKeymap,
    ...defaultKeymap,
  ];
}

function languageExtension(
  dialect: SqlEditorDialect,
  completionSchema: SqlEditorCompletionCache | undefined,
): Extension {
  return sql({
    dialect: dialects[dialect] ?? StandardSQL,
    schema: completionSchema ? buildSqlNamespace(completionSchema) : undefined,
    defaultSchema: completionSchema
      ? pickDefaultSchema(completionSchema)
      : undefined,
    upperCaseKeywords: true,
  });
}

function buildSqlNamespace(cache: SqlEditorCompletionCache): SQLNamespace {
  const database = pickDatabase(cache);
  if (!database) return {};

  const namespace: Record<string, SQLNamespace> = {};
  for (const schema of database.schemas) {
    const children: Record<string, SQLNamespace> = {};
    for (const table of schema.tables) {
      children[table.name] = relationNamespace(table, "table");
    }
    for (const view of schema.views) {
      children[view.name] = relationNamespace(view, "view");
    }
    namespace[schema.name] = {
      self: {
        label: schema.name,
        type: "namespace",
        detail: "schema",
      },
      children,
    };
  }
  return namespace;
}

function relationNamespace(
  relation: SqlEditorCompletionRelation,
  kind: "table" | "view",
): SQLNamespace {
  return {
    self: {
      label: relation.name,
      type: kind === "table" ? "class" : "interface",
      detail: `${relation.schema} ${kind}`,
      boost: kind === "table" ? 2 : 1,
    },
    children: relation.columns.map(columnCompletion),
  };
}

function columnCompletion(column: SqlEditorCompletionColumn): Completion {
  return {
    label: column.name,
    type: "property",
    detail: column.data_type
      ? `${column.data_type}${column.nullable === false ? " not null" : ""}`
      : "column",
  };
}

function pickDatabase(
  cache: SqlEditorCompletionCache,
): SqlEditorCompletionDatabase | undefined {
  return (
    cache.databases.find((database) => database.name === cache.database) ??
    cache.databases.find((database) => database.is_default) ??
    cache.databases[0]
  );
}

function pickDefaultSchema(cache: SqlEditorCompletionCache): string | undefined {
  const database = pickDatabase(cache);
  if (!database) return undefined;
  if (
    cache.defaultSchema &&
    database.schemas.some((schema) => schema.name === cache.defaultSchema)
  ) {
    return cache.defaultSchema;
  }
  return (
    database.schemas.find((schema) => schema.name === "public") ??
    database.schemas[0]
  )?.name;
}

function lineHighlightExtension(config: HighlightConfig): Extension {
  const field = StateField.define<LineHighlights>({
    create(state) {
      return buildLineHighlights(state.doc, config);
    },
    update(value, transaction) {
      return transaction.docChanged
        ? buildLineHighlights(transaction.state.doc, config)
        : value;
    },
    provide: (highlightField) => [
      EditorView.decorations.from(
        highlightField,
        (value) => value.decorations,
      ),
      gutterLineClass.from(highlightField, (value) => value.gutters),
    ],
  });

  return field;
}

interface LineHighlights {
  decorations: DecorationSet;
  gutters: ReturnType<RangeSetBuilder<GutterMarker>["finish"]>;
}

function buildLineHighlights(doc: Text, config: HighlightConfig): LineHighlights {
  const decorations = new RangeSetBuilder<Decoration>();
  const gutters = new RangeSetBuilder<GutterMarker>();

  if (config.statementRange) {
    const start = clampLine(doc, config.statementRange.startLine);
    const end = clampLine(doc, config.statementRange.endLine);
    for (let lineNo = start; lineNo <= end; lineNo++) {
      const line = doc.line(lineNo);
      decorations.add(line.from, line.from, statementLine);
      gutters.add(line.from, line.from, statementGutter);
    }
  }

  if (config.errorLine != null) {
    const line = doc.line(clampLine(doc, config.errorLine));
    decorations.add(line.from, line.from, errorLine);
    gutters.add(line.from, line.from, errorGutter);
  }

  return { decorations: decorations.finish(), gutters: gutters.finish() };
}

function clampLine(doc: Text, line: number): number {
  return Math.min(Math.max(1, line), doc.lines);
}

const cellarHighlightStyle = HighlightStyle.define([
  { tag: tags.keyword, color: "var(--syn-kw)", fontWeight: "600" },
  { tag: [tags.string, tags.special(tags.string)], color: "var(--syn-str)" },
  { tag: tags.number, color: "var(--syn-num)" },
  {
    tag: [tags.comment, tags.blockComment, tags.lineComment],
    color: "var(--syn-comment)",
  },
  {
    tag: [tags.name, tags.variableName, tags.propertyName],
    color: "var(--syn-ident)",
  },
  { tag: tags.operator, color: "var(--syn-op)" },
  { tag: tags.punctuation, color: "var(--fg-2)" },
  { tag: tags.invalid, color: "var(--syn-error)" },
]);

const cellarEditorTheme = EditorView.theme({
  "&": {
    height: "100%",
    color: "var(--fg-0)",
    backgroundColor: "var(--bg-inset)",
    fontFamily: "var(--font-mono)",
    fontSize: "12.5px",
  },
  ".cm-editor": {
    height: "100%",
  },
  ".cm-scroller": {
    fontFamily: "var(--font-mono)",
    lineHeight: "1.55",
    overflow: "auto",
  },
  ".cm-content": {
    minHeight: "100%",
    padding: "6px 16px 60px 0",
    caretColor: "var(--fg-0)",
  },
  ".cm-line": {
    padding: "0 0 0 8px",
  },
  ".cm-cursor": {
    borderLeftColor: "var(--fg-0)",
  },
  ".cm-selectionBackground, &.cm-focused .cm-selectionBackground": {
    backgroundColor: "var(--accent-line)",
  },
  ".cm-gutters": {
    padding: "6px 0 60px",
    color: "var(--fg-3)",
    backgroundColor: "transparent",
    border: "none",
  },
  ".cm-lineNumbers .cm-gutterElement": {
    minWidth: "48px",
    padding: "0 8px 0 4px",
    fontSize: "10.5px",
    fontVariantNumeric: "tabular-nums",
  },
  ".cm-activeLine": {
    backgroundColor: "color-mix(in oklab, var(--bg-3) 45%, transparent)",
  },
  ".cm-activeLineGutter": {
    color: "var(--fg-1)",
    backgroundColor: "transparent",
  },
  ".cm-cellar-current-statement": {
    background:
      "linear-gradient(90deg, var(--accent-soft), color-mix(in oklab, var(--accent-soft) 50%, transparent))",
  },
  ".cm-cellar-current-statement-gutter": {
    color: "var(--accent)",
    background:
      "linear-gradient(90deg, transparent, var(--accent-soft) 40%)",
  },
  ".cm-cellar-error-line": {
    textDecorationLine: "underline",
    textDecorationStyle: "wavy",
    textDecorationColor: "var(--syn-error)",
    textUnderlineOffset: "3px",
  },
  ".cm-cellar-error-line-gutter": {
    color: "var(--syn-error)",
  },
  ".cm-placeholder": {
    color: "var(--fg-4)",
  },
  ".cm-tooltip": {
    backgroundColor: "var(--bg-1)",
    border: "1px solid var(--border-strong)",
    borderRadius: "5px",
    boxShadow: "var(--shadow-md)",
    color: "var(--fg-1)",
  },
  ".cm-tooltip.cm-tooltip-autocomplete": {
    overflow: "hidden",
    padding: "2px 0",
    fontFamily: "var(--font-mono)",
    fontSize: "12px",
  },
  ".cm-tooltip-autocomplete > ul": {
    maxHeight: "240px",
  },
  ".cm-tooltip-autocomplete ul li": {
    minHeight: "22px",
    padding: "1px 8px 1px 6px",
    backgroundColor: "transparent",
    color: "var(--fg-1)",
  },
  ".cm-tooltip-autocomplete ul li[aria-selected]": {
    backgroundColor: "var(--accent-soft)",
    color: "var(--fg-0)",
  },
  ".cm-completionLabel": {
    color: "inherit",
  },
  ".cm-completionDetail": {
    marginLeft: "8px",
    color: "var(--fg-3)",
    fontStyle: "italic",
  },
  ".cm-tooltip-autocomplete ul li[aria-selected] .cm-completionDetail": {
    color: "var(--fg-2)",
  },
  ".cm-completionMatchedText": {
    color: "var(--accent)",
    fontWeight: "600",
    textDecoration: "none",
  },
  ".cm-completionIcon": {
    color: "var(--fg-3)",
    opacity: "0.8",
  },
  ".cm-tooltip-autocomplete ul li[aria-selected] .cm-completionIcon": {
    color: "var(--accent)",
    opacity: "1",
  },
  "&.cm-focused": {
    outline: "none",
  },
}, { dark: true });

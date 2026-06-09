import {
  autocompletion,
  closeBrackets,
  closeBracketsKeymap,
  type CompletionContext,
  type CompletionResult,
  completionKeymap,
} from "@codemirror/autocomplete";
import {
  defaultKeymap,
  history,
  historyKeymap,
  indentWithTab,
} from "@codemirror/commands";
import {
  MSSQL,
  MySQL,
  PostgreSQL,
  SQLite,
  StandardSQL,
  sql,
  type SQLDialect,
} from "@codemirror/lang-sql";
import {
  bracketMatching,
  HighlightStyle,
  foldGutter,
  indentOnInput,
  syntaxHighlighting,
} from "@codemirror/language";
import { highlightSelectionMatches, searchKeymap } from "@codemirror/search";
import {
  Compartment,
  EditorState,
  RangeSetBuilder,
  type Extension,
} from "@codemirror/state";
import {
  crosshairCursor,
  Decoration,
  drawSelection,
  dropCursor,
  EditorView,
  highlightActiveLine,
  highlightActiveLineGutter,
  keymap,
  lineNumbers,
  placeholder as editorPlaceholder,
  rectangularSelection,
  scrollPastEnd,
} from "@codemirror/view";
import { tags } from "@lezer/highlight";
import {
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  type KeyboardEvent,
  type MutableRefObject,
} from "react";
import { cellarCompletionSource } from "./completion";
import type { SqlDatabaseMeta, SqlEngine } from "./types";

export type {
  SqlColumnMeta,
  SqlDatabaseMeta,
  SqlEngine,
  SqlRelationMeta,
  SqlSchemaMeta,
} from "./types";

export interface SqlEditorProps {
  value: string;
  engine?: SqlEngine;
  databases?: readonly SqlDatabaseMeta[];
  database?: string | null;
  placeholder?: string;
  wrap?: boolean;
  showLineNumbers?: boolean;
  enableBracketMatching?: boolean;
  tabSize?: 2 | 4 | 8;
  currentStatementRange?: readonly [number, number] | null;
  errorLine?: number | null;
  className?: string;
  onChange: (value: string) => void;
  onCursorChange?: (offset: number) => void;
  onRunStatement?: (offset: number) => void;
  onRunAll?: () => void;
}

interface LatestCallbacks {
  onChange: SqlEditorProps["onChange"];
  onCursorChange?: SqlEditorProps["onCursorChange"];
  onRunStatement?: SqlEditorProps["onRunStatement"];
  onRunAll?: SqlEditorProps["onRunAll"];
}

const updateCompartment = new Compartment();
const languageCompartment = new Compartment();
const completionCompartment = new Compartment();
const wrappingCompartment = new Compartment();
const decorationCompartment = new Compartment();
const placeholderCompartment = new Compartment();
const lineNumbersCompartment = new Compartment();
const bracketMatchingCompartment = new Compartment();
const tabSizeCompartment = new Compartment();

export function SqlCodeEditor({
  value,
  engine = "postgres",
  databases = [],
  database = null,
  placeholder = "",
  wrap = false,
  showLineNumbers = true,
  enableBracketMatching = true,
  tabSize = 4,
  currentStatementRange = null,
  errorLine = null,
  className = "",
  onChange,
  onCursorChange,
  onRunStatement,
  onRunAll,
}: SqlEditorProps) {
  const host = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const latest = useRef<LatestCallbacks>({ onChange });

  latest.current = {
    onChange,
    onCursorChange,
    onRunStatement,
    onRunAll,
  };

  const completionSource = useMemo(
    () => cellarCompletionSource({ databases, database }),
    [databases, database],
  );

  useLayoutEffect(() => {
    if (!host.current || viewRef.current) return;

    const view = new EditorView({
      parent: host.current,
      state: EditorState.create({
        doc: value,
        extensions: [
          baseExtensions(latest),
          languageCompartment.of(languageExtension(engine)),
          completionCompartment.of(completionExtension(completionSource)),
          wrappingCompartment.of(wrap ? EditorView.lineWrapping : []),
          placeholderCompartment.of(editorPlaceholder(placeholder)),
          decorationCompartment.of(lineDecorations(currentStatementRange, errorLine)),
          updateCompartment.of(updateExtension(latest)),
          lineNumbersCompartment.of(showLineNumbers ? lineNumbers() : []),
          bracketMatchingCompartment.of(enableBracketMatching ? bracketMatching() : []),
          tabSizeCompartment.of(EditorState.tabSize.of(tabSize)),
        ],
      }),
    });

    viewRef.current = view;
    latest.current.onCursorChange?.(view.state.selection.main.head);

    return () => {
      view.destroy();
      viewRef.current = null;
    };
  }, []);

  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    const current = view.state.doc.toString();
    if (current === value) return;

    view.dispatch({
      changes: { from: 0, to: current.length, insert: value },
    });
  }, [value]);

  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    view.dispatch({
      effects: languageCompartment.reconfigure(languageExtension(engine)),
    });
  }, [engine]);

  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    view.dispatch({
      effects: completionCompartment.reconfigure(
        completionExtension(completionSource),
      ),
    });
  }, [completionSource]);

  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    view.dispatch({
      effects: wrappingCompartment.reconfigure(wrap ? EditorView.lineWrapping : []),
    });
  }, [wrap]);

  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    view.dispatch({
      effects: lineNumbersCompartment.reconfigure(showLineNumbers ? lineNumbers() : []),
    });
  }, [showLineNumbers]);

  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    view.dispatch({
      effects: bracketMatchingCompartment.reconfigure(enableBracketMatching ? bracketMatching() : []),
    });
  }, [enableBracketMatching]);

  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    view.dispatch({
      effects: tabSizeCompartment.reconfigure(EditorState.tabSize.of(tabSize)),
    });
  }, [tabSize]);

  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    view.dispatch({
      effects: placeholderCompartment.reconfigure(editorPlaceholder(placeholder)),
    });
  }, [placeholder]);

  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    view.dispatch({
      effects: decorationCompartment.reconfigure(
        lineDecorations(currentStatementRange, errorLine),
      ),
    });
  }, [currentStatementRange, errorLine]);

  return (
    <div
      ref={host}
      className={["cellar-cm", className].filter(Boolean).join(" ")}
      onKeyDown={stopHandledRunKeys}
    />
  );
}

function baseExtensions(latest: MutableRefObject<LatestCallbacks>): Extension {
  return [
    highlightActiveLineGutter(),
    history(),
    foldGutter(),
    drawSelection(),
    dropCursor(),
    EditorState.allowMultipleSelections.of(true),
    indentOnInput(),
    syntaxHighlighting(sqlHighlightStyle, { fallback: true }),
    closeBrackets(),
    rectangularSelection(),
    crosshairCursor(),
    highlightActiveLine(),
    highlightSelectionMatches(),
    scrollPastEnd(),
    keymap.of([
      {
        key: "Mod-Enter",
        run(view) {
          latest.current.onRunStatement?.(view.state.selection.main.head);
          return true;
        },
      },
      {
        key: "Mod-Shift-Enter",
        run() {
          latest.current.onRunAll?.();
          return true;
        },
      },
      indentWithTab,
      ...closeBracketsKeymap,
      ...completionKeymap,
      ...searchKeymap,
      ...historyKeymap,
      ...defaultKeymap,
    ]),
  ];
}

const sqlHighlightStyle = HighlightStyle.define([
  { tag: tags.keyword, class: "cm-cellar-token-keyword" },
  { tag: tags.operatorKeyword, class: "cm-cellar-token-keyword" },
  { tag: tags.function(tags.variableName), class: "cm-cellar-token-function" },
  { tag: tags.standard(tags.variableName), class: "cm-cellar-token-function" },
  { tag: tags.string, class: "cm-cellar-token-string" },
  { tag: tags.number, class: "cm-cellar-token-number" },
  { tag: tags.comment, class: "cm-cellar-token-comment" },
  { tag: tags.operator, class: "cm-cellar-token-operator" },
  { tag: tags.variableName, class: "cm-cellar-token-identifier" },
  { tag: tags.name, class: "cm-cellar-token-identifier" },
]);

function updateExtension(latest: MutableRefObject<LatestCallbacks>): Extension {
  return EditorView.updateListener.of((update) => {
    if (update.docChanged) {
      latest.current.onChange(update.state.doc.toString());
    }
    if (update.docChanged || update.selectionSet) {
      latest.current.onCursorChange?.(update.state.selection.main.head);
    }
  });
}

function languageExtension(engine: SqlEngine): Extension {
  return sql({
    dialect: dialectFor(engine),
    upperCaseKeywords: true,
  });
}

function completionExtension(source: (context: CompletionContext) => CompletionResult | null) {
  return autocompletion({
    activateOnTyping: true,
    maxRenderedOptions: 80,
    override: [source],
  });
}

function lineDecorations(
  range: readonly [number, number] | null,
  errorLine: number | null,
): Extension {
  return EditorView.decorations.of((view) => {
    const builder = new RangeSetBuilder<Decoration>();
    const lines = view.state.doc.lines;
    const classes = new Map<number, string[]>();

    if (range) {
      const fromLine = clampLine(range[0], lines);
      const toLine = clampLine(range[1], lines);
      for (let lineNo = fromLine; lineNo <= toLine; lineNo++) {
        appendLineClass(classes, lineNo, "cm-cellar-current-statement");
      }
    }

    if (errorLine != null) {
      appendLineClass(classes, clampLine(errorLine, lines), "cm-cellar-error-line");
    }

    for (const [lineNo, lineClasses] of [...classes].sort((a, b) => a[0] - b[0])) {
      const line = view.state.doc.line(lineNo);
      builder.add(
        line.from,
        line.from,
        Decoration.line({ class: lineClasses.join(" ") }),
      );
    }

    return builder.finish();
  });
}

function appendLineClass(
  classes: Map<number, string[]>,
  lineNo: number,
  className: string,
) {
  const existing = classes.get(lineNo) ?? [];
  existing.push(className);
  classes.set(lineNo, existing);
}

function clampLine(line: number, max: number) {
  return Math.max(1, Math.min(max, line));
}

function dialectFor(engine: SqlEngine): SQLDialect {
  switch (engine) {
    case "postgres":
      return PostgreSQL;
    case "mysql":
      return MySQL;
    case "sqlite":
      return SQLite;
    case "mssql":
    case "azure":
      return MSSQL;
    default:
      return StandardSQL;
  }
}

function stopHandledRunKeys(event: KeyboardEvent<HTMLDivElement>) {
  const mod = event.metaKey || event.ctrlKey;
  if (mod && event.key === "Enter") {
    event.stopPropagation();
  }
}

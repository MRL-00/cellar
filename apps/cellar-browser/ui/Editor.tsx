import { useEffect, useRef } from 'react';
import { basicSetup } from 'codemirror';
import { EditorView, keymap } from '@codemirror/view';
import { sql, PostgreSQL, SQLite } from '@codemirror/lang-sql';
import {HighlightStyle,syntaxHighlighting} from '@codemirror/language';
import {tags} from '@lezer/highlight';
import type { Table } from './types.js';

export function Editor({ value, engine, tables, onChange, onRun }: { value: string; engine: string; tables: Table[]; onChange: (sql: string) => void; onRun: (selection?: string,cursor?:number) => void }) {
  const element = useRef<HTMLDivElement>(null);
  const view = useRef<EditorView>();
  const callbacks = useRef({ onChange, onRun });
  callbacks.current = { onChange, onRun };
  useEffect(() => {
    const editor = new EditorView({
      parent: element.current!, doc: value,
      extensions: [basicSetup, sql({ dialect: ['postgres','supabase','neon'].includes(engine) ? PostgreSQL : SQLite, schema: Object.fromEntries(tables.map(table => [table.name, table.columns.map(column => column.name)])) }),
        syntaxHighlighting(HighlightStyle.define([{tag:tags.keyword,class:'sql-keyword'},{tag:tags.string,class:'sql-string'},{tag:tags.number,class:'sql-number'},{tag:tags.comment,class:'sql-comment'},{tag:tags.operator,class:'sql-operator'}])),
        EditorView.theme({ '&': { height: '100%', backgroundColor: 'var(--inset)', color: 'var(--fg)' }, '.cm-scroller': { overflow: 'auto', fontFamily: 'var(--mono)', fontSize: '13px' }, '.cm-gutters': { backgroundColor: 'var(--panel)', color: 'var(--muted)', border: 'none' }, '.cm-content': { caretColor: 'var(--accent)' } }, { dark: true }),
        EditorView.updateListener.of(update => { if (update.docChanged) callbacks.current.onChange(update.state.doc.toString()); }),
        keymap.of([{ key: 'Mod-Enter', run: current => { const selection = current.state.selection.main; callbacks.current.onRun(selection.empty ? undefined : current.state.sliceDoc(selection.from, selection.to),new TextEncoder().encode(current.state.sliceDoc(0,selection.head)).length); return true; } }]),
      ],
    });
    view.current = editor;
    return () => editor.destroy();
  }, [engine, tables]);
  useEffect(() => {
    const editor = view.current;
    if (editor && editor.state.doc.toString() !== value) editor.dispatch({ changes: { from: 0, to: editor.state.doc.length, insert: value } });
  }, [value]);
  return <div className="editor" ref={element} aria-label="SQL editor" />;
}

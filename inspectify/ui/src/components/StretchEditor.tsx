import React from "react";
import * as monaco from "monaco-editor";

import "monaco-editor/esm/vs/editor/editor.all.js";

export type AnalysisError = {
  loc: { start: number; end: number };
  kind: "Sat" | "Unknown";
  reason: string | void;
};

export const StretchEditor = ({
  source,
  onChange,
  errors = [],
  language = gclId,
}: {
  source: string;
  onChange?: (source: string) => void;
  errors?: AnalysisError[];
  language?: string;
}) => {
  const [model] = React.useState(
    monaco.editor.createModel(source || "", language)
  );

  const [container, setContainer] = React.useState<HTMLDivElement | null>(null);
  const editorRef = React.useRef<monaco.editor.IStandaloneCodeEditor | null>(
    null
  );

  React.useEffect(() => {
    if (container) {
      const editor = (editorRef.current = monaco.editor.create(container, {
        model,
        theme: themeName,
        minimap: { enabled: false },
        smoothScrolling: true,
        readOnly: !onChange,
        lineNumbers: "on",
        scrollBeyondLastLine: false,
        folding: false,
        quickSuggestions: false,
        language: language,
        wordWrap: "bounded",
        renderLineHighlightOnlyWhenFocus: true,
        scrollbar: {
          vertical: "auto",
          verticalScrollbarSize: 0,
        },
      }));

      editor.createDecorationsCollection([
        {
          range: new monaco.Range(4, 2, 4, 5),
          options: {
            isWholeLine: false,
            inlineClassName: "someClassName",
          },
        },
      ]);

      return () => {
        editor.dispose();
      };
    }
  }, [container]);

  React.useEffect(() => {
    const editor = editorRef.current;
    if (editor) editor.setModel(model);
  }, [model, editorRef.current]);

  React.useEffect(() => {
    const listener = () => {
      const editor = editorRef.current;
      if (editor) editor.layout();
    };

    if (editorRef.current) {
      editorRef.current.layout();
      editorRef.current.layout();
    }

    window.addEventListener("resize", listener);
    return () => window.removeEventListener("resize", listener);
  }, [editorRef.current]);

  React.useEffect(() => {
    const r = model.onDidChangeContent(
      () => onChange && onChange(model.getValue())
    );

    return () => r.dispose();
  }, [model, onChange]);

  React.useEffect(() => {
    if (source && model.getValue() != source) model.setValue(source);
  }, [onChange, source]);

  React.useEffect(() => {
    const i = setInterval(() => {
      if (editorRef.current) {
        editorRef.current.layout();
        editorRef.current.layout();
      }
    }, 200);

    return () => clearInterval(i);
  }, []);

  if (editorRef.current) {
    const decs = errors.map<monaco.editor.IMarkerData>((error) => {
      const toLoc = (idx: number) => {
        const isOnNewline = source[idx] == "\n";
        const lines = source.slice(0, idx + 1).split("\n");
        const offset = lines[lines.length - 1]!.length;
        return { line: lines.length, col: isOnNewline ? offset + 1 : offset };
      };
      const start = toLoc(error.loc.start);
      const end = toLoc(error.loc.end);

      return {
        severity:
          error.kind == "Sat"
            ? monaco.MarkerSeverity.Error
            : monaco.MarkerSeverity.Warning,
        message:
          (error.kind == "Sat" ? "Will not hold" : "Might not hold") +
          (error.reason ? `:\n${error.reason}` : ""),
        startLineNumber: start.line,
        startColumn: start.col,
        endLineNumber: end.line,
        endColumn: end.col,
      };
    });

    monaco.editor.setModelMarkers(editorRef.current.getModel()!, "gcl", decs);
  }

  return <div className="absolute inset-0" ref={setContainer} />;
};

const gclId = "gcl";

monaco.languages.register({
  id: gclId,
  extensions: ["gcl"],
  aliases: [],
  mimetypes: ["application/gcl"],
});
monaco.languages.setLanguageConfiguration(gclId, {
  comments: {
    lineComment: "//",
    blockComment: ["/*", "*/"],
  },
  brackets: [
    ["(", ")"],
    ["{", "}"],
    ["[", "]"],
  ],
  autoClosingPairs: [
    { open: "[", close: "]" },
    { open: "{", close: "}" },
    { open: "(", close: ")" },
    { open: "'", close: "'", notIn: ["string", "comment"] },
    { open: '"', close: '"', notIn: ["string"] },
  ],
  surroundingPairs: [
    { open: "{", close: "}" },
    { open: "[", close: "]" },
    { open: "(", close: ")" },
    { open: '"', close: '"' },
    { open: "'", close: "'" },
  ],
  folding: {
    markers: {
      start: new RegExp("^\\s*#pragma\\s+region\\b"),
      end: new RegExp("^\\s*#pragma\\s+endregion\\b"),
    },
  },
  wordPattern: /[a-zA-Z_@$ΣΛλ][a-zA-Z0-9_]*/,
});
monaco.languages.setMonarchTokensProvider(gclId, {
  defaultToken: "",
  brackets: [
    { token: "delimiter.curly", open: "{", close: "}" },
    { token: "delimiter.parenthesis", open: "(", close: ")" },
    { token: "delimiter.square", open: "[", close: "]" },
    { token: "delimiter.angle", open: "<", close: ">" },
  ],

  keywords: ["if", "fi", "do", "od"],
  operators: [
    "-",
    ",",
    "->",
    ":=",
    "!",
    "!=",
    "(",
    ")",
    "{",
    "}",
    "*",
    "/",
    "^",
    "&&",
    "&",
    "+",
    "<",
    "<=",
    "=",
    ">",
    ">=",
    "||",
    "|",
  ],
  tokenizer: {
    root: [
      [
        /[a-zA-Z_@$ΣΛλ][a-zA-Z0-9_]*/,
        {
          cases: {
            "@keywords": "keyword",
            "@operators": "operator",
            "@default": "identifier",
          },
        },
      ],
      { include: "@whitespace" },
      [/[-,:=!*\/&+<>|]/, "keyword.operator"],
      [/(\/\/).*$/, "comment"],
      [/[{}()\[\]]/, "@brackets"],
      [/[0-9]+/, "number"],
    ],
    whitespace: [
      [/[ \t\r\n]+/, ""],
      [/\/\*/, "comment", "@comment"],
      [/\/\/.*\\$/, "comment", "@linecomment"],
      [/\/\/.*$/, "comment"],
    ],
    comment: [
      [/[^\/*]+/, "comment"],
      [/\*\//, "comment", "@pop"],
      [/[\/*]/, "comment"],
    ],
    linecomment: [
      [/.*[^\\]$/, "comment", "@pop"],
      [/[^]+/, "comment"],
    ],
  },
});

// Theme definition

// Theme is "Tomorrow-Night-Eighties.json" taken from https://github.com/brijeshb42/monaco-themes/blob/master/themes/Tomorrow.json
// Preview themes at https://editor.bitwiser.in
const themeName = "Tomorrow-Night-Eighties";
const theme: monaco.editor.IStandaloneThemeData = {
  base: "vs-dark",
  inherit: true,
  rules: [
    {
      background: "200020",
      token: "",
    },
    {
      foreground: "404080",
      background: "200020",
      fontStyle: "italic",
      token: "comment.block",
    },
    {
      foreground: "999999",
      token: "string",
    },
    {
      foreground: "707090",
      token: "constant.language",
    },
    {
      foreground: "7090b0",
      token: "constant.numeric",
    },
    {
      fontStyle: "bold",
      token: "constant.numeric.integer.int32",
    },
    {
      fontStyle: "italic",
      token: "constant.numeric.integer.int64",
    },
    {
      fontStyle: "bold italic",
      token: "constant.numeric.integer.nativeint",
    },
    {
      fontStyle: "underline",
      token: "constant.numeric.floating-point.ocaml",
    },
    {
      foreground: "666666",
      token: "constant.character",
    },
    {
      foreground: "8080a0",
      token: "constant.language.boolean",
    },
    {
      foreground: "008080",
      token: "variable.language",
    },
    {
      foreground: "008080",
      token: "variable.other",
    },
    {
      foreground: "a080ff",
      token: "keyword",
    },
    {
      foreground: "a0a0ff",
      token: "keyword.operator",
    },
    {
      foreground: "d0d0ff",
      token: "keyword.other.decorator",
    },
    {
      fontStyle: "underline",
      token: "keyword.operator.infix.floating-point.ocaml",
    },
    {
      fontStyle: "underline",
      token: "keyword.operator.prefix.floating-point.ocaml",
    },
    {
      foreground: "c080c0",
      token: "keyword.other.directive",
    },
    {
      foreground: "c080c0",
      fontStyle: "underline",
      token: "keyword.other.directive.line-number",
    },
    {
      foreground: "80a0ff",
      token: "keyword.control",
    },
    {
      foreground: "b0fff0",
      token: "storage",
    },
    {
      foreground: "60b0ff",
      token: "entity.name.type.variant",
    },
    {
      foreground: "60b0ff",
      fontStyle: "italic",
      token: "storage.type.variant.polymorphic",
    },
    {
      foreground: "60b0ff",
      fontStyle: "italic",
      token: "entity.name.type.variant.polymorphic",
    },
    {
      foreground: "b000b0",
      token: "entity.name.type.module",
    },
    {
      foreground: "b000b0",
      fontStyle: "underline",
      token: "entity.name.type.module-type.ocaml",
    },
    {
      foreground: "a00050",
      token: "support.other",
    },
    {
      foreground: "70e080",
      token: "entity.name.type.class",
    },
    {
      foreground: "70e0a0",
      token: "entity.name.type.class-type",
    },
    {
      foreground: "50a0a0",
      token: "entity.name.function",
    },
    {
      foreground: "80b0b0",
      token: "variable.parameter",
    },
    {
      foreground: "3080a0",
      token: "entity.name.type.token",
    },
    {
      foreground: "3cb0d0",
      token: "entity.name.type.token.reference",
    },
    {
      foreground: "90e0e0",
      token: "entity.name.function.non-terminal",
    },
    {
      foreground: "c0f0f0",
      token: "entity.name.function.non-terminal.reference",
    },
    {
      foreground: "009090",
      token: "entity.name.tag",
    },
    {
      background: "200020",
      token: "support.constant",
    },
    {
      foreground: "400080",
      background: "ffff00",
      fontStyle: "bold",
      token: "invalid.illegal",
    },
    {
      foreground: "200020",
      background: "cc66ff",
      token: "invalid.deprecated",
    },
    {
      background: "40008054",
      token: "source.camlp4.embedded",
    },
    {
      foreground: "805080",
      token: "punctuation",
    },
  ],
  colors: {
    "editor.foreground": "#D0D0FF",
    "editor.background": "#1e293b",
    "editor.selectionBackground": "#80000080",
    "editor.lineHighlightBackground": "#80000040",
    "editorCursor.foreground": "#7070FF",
    "editorWhitespace.foreground": "#BFBFBF",
  },
};
monaco.editor.defineTheme(themeName, theme);

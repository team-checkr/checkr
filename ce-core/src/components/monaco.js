const initializeMonaco = () => {
  if (!window.leMonaco) {
    window.leMonaco = new Promise((res) => {
      require.config({
        paths: {
          vs: "https://cdn.jsdelivr.net/npm/monaco-editor@0.22.3/min/vs",
        },
      });
      require(["vs/editor/editor.main"], function () {
        setTimeout(() => {
          res(window.monaco);
        }, 100);
      });
    });
  }
  return window.leMonaco;
};

const monaco = await initializeMonaco();

const id = "%id%";
const value = "%value%";
const container = document.getElementById(id);

let run = async () => {
  if (!window.editors) window.editors = {};
  const editor = (window.editors[id] =
    window.editors[id] ||
    monaco.editor.create(container, {
      value: value,
      language: gclId,
      theme: themeName,
      minimap: { enabled: false },
      smoothScrolling: true,
      // readOnly: !onChange,
      lineNumbers: "on",
      scrollBeyondLastLine: false,
      folding: false,
      quickSuggestions: false,
      wordWrap: "bounded",
      renderLineHighlightOnlyWhenFocus: true,
      scrollbar: {
        vertical: "auto",
        verticalScrollbarSize: 0,
      },
    }));
  editor.layout();
  const model = editor.getModel();
  window.listeners = window.listeners || {};
  if (window.listeners[id]) {
    window.listeners[id].dispose();
  }
  window.listeners[id] = model.onDidChangeContent(() => {
    console.log({ id }, "did change");
    dioxus.send(model.getValue());
  });
  if (model.getValue() != value) {
    window.listeners[id].dispose();
    editor.setValue(value);
    window.listeners[id] = model.onDidChangeContent(() => {
      dioxus.send(model.getValue());
    });
  }
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
const theme = {
  base: "vs-dark",
  inherit: true,
  rules: [
    {
      background: "#200020",
      token: "",
    },
    {
      foreground: "#404080",
      background: "#200020",
      fontStyle: "italic",
      token: "comment.block",
    },
    {
      foreground: "#999999",
      token: "string",
    },
    {
      foreground: "#707090",
      token: "constant.language",
    },
    {
      foreground: "#7090b0",
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
      foreground: "#666666",
      token: "constant.character",
    },
    {
      foreground: "#8080a0",
      token: "constant.language.boolean",
    },
    {
      foreground: "#008080",
      token: "variable.language",
    },
    {
      foreground: "#008080",
      token: "variable.other",
    },
    {
      foreground: "#a080ff",
      token: "keyword",
    },
    {
      foreground: "#a0a0ff",
      token: "keyword.operator",
    },
    {
      foreground: "#d0d0ff",
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
      foreground: "#c080c0",
      token: "keyword.other.directive",
    },
    {
      foreground: "#c080c0",
      fontStyle: "underline",
      token: "keyword.other.directive.line-number",
    },
    {
      foreground: "#80a0ff",
      token: "keyword.control",
    },
    {
      foreground: "#b0fff0",
      token: "storage",
    },
    {
      foreground: "#60b0ff",
      token: "entity.name.type.variant",
    },
    {
      foreground: "#60b0ff",
      fontStyle: "italic",
      token: "storage.type.variant.polymorphic",
    },
    {
      foreground: "#60b0ff",
      fontStyle: "italic",
      token: "entity.name.type.variant.polymorphic",
    },
    {
      foreground: "#b000b0",
      token: "entity.name.type.module",
    },
    {
      foreground: "#b000b0",
      fontStyle: "underline",
      token: "entity.name.type.module-type.ocaml",
    },
    {
      foreground: "#a00050",
      token: "support.other",
    },
    {
      foreground: "#70e080",
      token: "entity.name.type.class",
    },
    {
      foreground: "#70e0a0",
      token: "entity.name.type.class-type",
    },
    {
      foreground: "#50a0a0",
      token: "entity.name.function",
    },
    {
      foreground: "#80b0b0",
      token: "variable.parameter",
    },
    {
      foreground: "#3080a0",
      token: "entity.name.type.token",
    },
    {
      foreground: "#3cb0d0",
      token: "entity.name.type.token.reference",
    },
    {
      foreground: "#90e0e0",
      token: "entity.name.function.non-terminal",
    },
    {
      foreground: "#c0f0f0",
      token: "entity.name.function.non-terminal.reference",
    },
    {
      foreground: "#009090",
      token: "entity.name.tag",
    },
    {
      background: "#200020",
      token: "support.constant",
    },
    {
      foreground: "#400080",
      background: "#ffff00",
      fontStyle: "bold",
      token: "invalid.illegal",
    },
    {
      foreground: "#200020",
      background: "#cc66ff",
      token: "invalid.deprecated",
    },
    {
      background: "#40008054",
      token: "source.camlp4.embedded",
    },
    {
      foreground: "#805080",
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

run();

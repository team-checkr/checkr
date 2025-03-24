<script lang="ts">
  import Katex from '$lib/components/Katex.svelte';

  type Production = {
    left: string;
    right: string[][];
    inline?: boolean;
  };

  /*
  BExpr = {
    AExpr RelOp AExpr,
    "true",
    "false",
    "!" BExpr,
    "(" BExpr ")",

    BExpr "&" BExpr,

    BExpr "|" BExpr,
};

RelOp = {
    "<",
    ">",
    "<=",
    ">=",
    "=",
    "!=",
};

Var = r"[_a-zA-Z][_a-zA-Z0-9]*",

Command = PredicateBlock* CommandKind PredicateBlock*;

CommandKind = {
    Var ":=" AExpr,
    "skip",
    "if" Guard "fi",
    "do" PredicateInv Guard "od",
    Command "[]" Command,
};

Guard = {
    BExpr "->" Command,
    Guard "[]" Guard,
};

PredicateBlock = "{" Predicate "}";

PredicateInv = "[" Predicate "]";

Predicate = {
    AExpr RelOp AExpr,
    "true",
    "false",
    "!" Predicate,
    "(" Predicate ")",

    Predicate "&" Predicate,

    Predicate "|" Predicate,

    Predicate "==>" Predicate,

    Quantifier Var "::" Predicate,
};

Quantifier = {
    "exists",
    "forall",
};

AExpr = {
    Int,
    Var,
    "-" <AExpr>,
    "(" <AExpr> ")",
    Function,

    AExpr "*" AExpr,
    AExpr "/" AExpr,

    AExpr "+" AExpr,
    AExpr "-" AExpr,
};

Function = {
    "division" "(" AExpr "," AExpr ")",
    "min" "(" AExpr "," AExpr ")",
    "max" "(" AExpr "," AExpr ")",
    "fac" "(" AExpr ")",
    "fib" "(" AExpr ")",
    "exp" "(" AExpr "," AExpr ")",
};
*/

  const productions: Production[] = [
    {
      left: 'BExpr',
      right: [
        ['AExpr', 'RelOp', 'AExpr'],
        ['"true"'],
        ['"false"'],
        ['"!"', 'BExpr'],
        ['"("', 'BExpr', '")"'],
        ['BExpr', '"&"', 'BExpr'],
        ['BExpr', '"|"', 'BExpr'],
      ],
    },
    {
      left: 'RelOp',
      right: [['"<"'], ['">"'], ['"<="'], ['">="'], ['"="'], ['"!="']],
      inline: true,
    },
    {
      left: 'Var',
      right: [['r"[\\_a-zA-Z][\\_a-zA-Z0-9]*"']],
    },
    {
      left: 'Command',
      right: [['PredicateBlock', '*', 'CommandKind', 'PredicateBlock', '*']],
    },
    {
      left: 'CommandKind',
      right: [
        ['Var', '":="', 'AExpr'],
        ['"skip"'],
        ['"if"', 'Guard', '"fi"'],
        ['"do"', 'PredicateInv', 'Guard', '"od"'],
        ['Command', '"[]"', 'Command'],
      ],
    },
    {
      left: 'Guard',
      right: [
        ['BExpr', '"->"', 'Command'],
        ['Guard', '"[]"', 'Guard'],
      ],
    },
    {
      left: 'PredicateBlock',
      right: [['"{"', 'Predicate', '"}"']],
    },
    {
      left: 'PredicateInv',
      right: [['"["', 'Predicate', '"]"']],
    },
    {
      left: 'Predicate',
      right: [
        ['AExpr', 'RelOp', 'AExpr'],
        ['"true"'],
        ['"false"'],
        ['"!"', 'Predicate'],
        ['"("', 'Predicate', '")"'],
        ['Predicate', '"&"', 'Predicate'],
        ['Predicate', '"|"', 'Predicate'],
        ['Predicate', '"==>"', 'Predicate'],
        ['Quantifier', 'Var', '"::"', 'Predicate'],
      ],
    },
    {
      left: 'Quantifier',
      right: [['"exists"'], ['"forall"']],
      inline: true,
    },
    {
      left: 'AExpr',
      right: [
        ['Int'],
        ['Var'],
        ['"-"', '<AExpr>'],
        ['"("', '<AExpr>', '")"'],
        ['AExpr', '"*"', 'AExpr'],
        ['AExpr', '"/"', 'AExpr'],
        ['AExpr', '"+"', 'AExpr'],
        ['AExpr', '"-"', 'AExpr'],
        ['Function'],
      ],
    },
    {
      left: 'Function',
      right: [
        ['"division"', '"("', 'AExpr', '","', 'AExpr', '")"'],
        ['"min"', '"("', 'AExpr', '","', 'AExpr', '")"'],
        ['"max"', '"("', 'AExpr', '","', 'AExpr', '")"'],
        ['"fac"', '"("', 'AExpr', '")"'],
        ['"fib"', '"("', 'AExpr', '")"'],
        ['"exp"', '"("', 'AExpr', '","', 'AExpr', '")"'],
      ],
    },
  ];

  const pascalCaseToKebabCase = (str: string) =>
    str.replace(/([a-z0-9])([A-Z])/g, '$1-$2').toLowerCase();

  const prepareToken = (token: string) => {
    // replace "&" with "\&"
    token = token.replace(/&/g, '\\&');

    // replace "{" and "}" with "\{" and "\}"
    token = token.replace(/{/g, '\\{').replace(/}/g, '\\}');

    // replace "*$" with "^*"
    token = token.replace(/\*$/g, '^*');

    // make keywords starting and ending with " texttt
    if (token.match(/".*"/g)) {
      return `\\;\\texttt{${token}}\\;`;
    }

    // make regex bold
    if (token.match(/r"[_a-zA-Z][_a-zA-Z0-9]*"/)) {
      return `\\texttt{${token}}`;
    }

    // make non-terminals italic
    if (token.match(/[A-Z][a-zA-Z]*/)) {
      return `\\langle \\textit{${pascalCaseToKebabCase(token)}} \\rangle`;
    }

    return token;
  };

  const grammar = `
\\begin{aligned}
    ${productions
      .map(
        (production) =>
          prepareToken(production.left) +
          ' ::= & \\;' +
          (production.inline
            ? production.right.map((right) => right.map(prepareToken).join(' ')).join(' \\mid  \\;')
            : production.right
                .map((right) => right.map(prepareToken).join(' '))
                .join(' \\\\  \\mid  & \\;')) +
          ' \\\\',
      )
      .join('')}
\\end{aligned}
  `;
</script>

<svelte:head>
  <title>Chip – Guide</title>
  <meta name="description" content="Guide" />
</svelte:head>

<div class="bg-slate-800 py-10">
  <article class="prose prose-invert mx-auto">
    <h1>Guide</h1>

    Chip is a tool for program verification.

    <h2>Grammar</h2>

    <Katex math={grammar} displayMode={true} />
  </article>
</div>

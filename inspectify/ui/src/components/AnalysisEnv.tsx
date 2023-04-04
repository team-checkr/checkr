import React, { useEffect, useState } from "react";
import * as core from "../lib/core";
import {
  ArrowPathRoundedSquareIcon,
  ClipboardDocumentIcon,
} from "@heroicons/react/24/outline";
import * as api from "../lib/api";
import ReactMarkdown from "react-markdown";
import { parse } from "ansicolor";
import remarkGfm from "remark-gfm";
import {
  Analysis,
  AnalysisResponse,
  CompilationStatus,
  Input,
} from "../lib/types";
import { StretchEditor } from "./StretchEditor";
import { Indicator, IndicatorState, INDICATOR_TEXT_COLOR } from "./Indicator";
import { capitalCase } from "change-case";
import { toast, Toaster } from "react-hot-toast";
import {
  QueryClient,
  QueryClientProvider,
  useMutation,
  useQuery,
} from "react-query";

const searchParams = new URL(document.location.toString()).searchParams;

const inputted: { analysis?: string; src?: string; input?: string } =
  Object.fromEntries(searchParams.entries());

const ENVS = Object.values(Analysis).filter(
  (a) => a != Analysis.Graph
) satisfies Analysis[];
const ANALYSIS_NAMES = Object.fromEntries(
  ENVS.map((env) => [env, capitalCase(env)] satisfies [Analysis, string])
) as Record<Analysis, string>;

type GraphShown = "graph" | "reference" | "split";

const queryClient = new QueryClient();

export const AnalysisEnv = () => {
  return (
    <QueryClientProvider client={queryClient}>
      <AnalysisEnvInner />
    </QueryClientProvider>
  );
};
export const AnalysisEnvInner = () => {
  const [deterministic, setDeterministic] = useState(true);
  const [env, setEnv] = useState<Analysis>(
    inputted.analysis && (ENVS as string[]).includes(inputted.analysis)
      ? (inputted.analysis as Analysis)
      : Analysis.Parse
  );
  const [src, setSrc] = useState(inputted.src ?? "skip");
  const [graphShown, setGraphShown] = useState<GraphShown>("graph");

  const { data: dotReference } = useQuery(
    ["dot-reference", deterministic, src],
    async ({}) => core.dot(deterministic, src),
    { keepPreviousData: true }
  );

  const compilationStatus = api.useCompilationStatus({ queryClient });

  const { data: dotGraph } = useQuery(
    [
      ["dotGraph", compilationStatus?.state, deterministic, src],
      deterministic,
      src,
    ],
    ({ signal }) => api.graph(signal, { deterministic, src }),
    { keepPreviousData: true }
  );

  const generateProgram = useMutation(
    async (env: Analysis) => {
      const src = await core.generate_program(env);
      const input = await core.generate_input_for(src, env);
      return { src, input };
    },
    {
      onSuccess: ({ src, input }) => {
        setSrc(src);
        setInput(input ?? null);
      },
    }
  );
  const [input, setInput] = useState<Input | null>(null);

  return (
    <div className="grid min-h-0 grid-cols-[1fr_2fr] grid-rows-[1fr_auto_auto_1fr]">
      <div className="grid grid-rows-[auto_1fr] divide-y divide-slate-600">
        <div className="grid grid-cols-3 divide-x divide-slate-600 border-r border-slate-600">
          <button
            className="flex items-center justify-center space-x-1 bg-slate-800 py-1 px-1.5 text-sm text-white transition hover:bg-slate-700 active:bg-slate-900"
            onClick={() => generateProgram.mutate(env)}
          >
            <span>Generate</span>
            <ArrowPathRoundedSquareIcon className="w-4" />
          </button>
          <label
            htmlFor="determinism"
            className="flex select-none items-center justify-center space-x-2 bg-slate-800 py-1 px-1.5 text-sm text-white transition hover:bg-slate-700 active:bg-slate-900"
          >
            <span>Determinism</span>
            <input
              type="checkbox"
              name="determinism"
              id="determinism"
              checked={deterministic}
              onChange={(e) => setDeterministic(e.target.checked)}
            />
          </label>
          <select
            className="flex appearance-none items-center justify-center space-x-1 rounded-none bg-slate-800 py-1 px-1.5 text-center text-sm text-white transition hover:bg-slate-700 active:bg-slate-900"
            value={env}
            onChange={(e) => setEnv(e.target.value as Analysis)}
          >
            {ENVS.map((e) => (
              <option key={e} value={e}>
                {ANALYSIS_NAMES[e]}
              </option>
            ))}
          </select>
        </div>
        <div className="relative">
          <StretchEditor source={src} onChange={setSrc} />
        </div>
      </div>
      <div className="relative row-span-2 bg-slate-800 text-xs">
        <div className="absolute top-4 right-6 flex flex-col space-y-2">
          {(
            [
              {
                graph: "graph",
                icon: "G",
                dot:
                  dotGraph?.type == "Graph"
                    ? dotGraph.content.dot
                    : dotGraph?.type == "Error"
                    ? dotGraph.content.error
                    : "",
              },
              { graph: "reference", icon: "R", dot: dotReference ?? "" },
              { graph: "split", icon: "S", dot: null },
            ] satisfies {
              graph: GraphShown;
              icon: string;
              dot: string | null;
            }[]
          ).map((g) => (
            <div key={g.graph} className="flex flex-row-reverse">
              <button
                onClick={() => setGraphShown(g.graph)}
                className={
                  "z-10 flex aspect-square w-7 items-center justify-center rounded-full border border-current p-1 text-center transition hover:text-slate-200 " +
                  (graphShown == g.graph ? "text-white" : "text-slate-600")
                }
              >
                {g.icon}
              </button>
              {g.dot && (
                <button
                  onClick={() => {
                    if (!g.dot) return;
                    toast("Graph DOT copied to clipboard!");
                    window.navigator.clipboard.writeText(g.dot);
                  }}
                  className={
                    "z-10 flex aspect-square w-7 items-center justify-center p-1 transition text-slate-500 hover:text-slate-200 mr-2" +
                    (graphShown == g.graph ? "text-white" : "text-slate-600")
                  }
                >
                  <ClipboardDocumentIcon className="w-4" />
                </button>
              )}
            </div>
          ))}
        </div>
        {graphShown == "graph" ? (
          dotGraph?.type == "Graph" && <Network dot={dotGraph.content.dot} />
        ) : graphShown == "reference" ? (
          dotReference && <Network dot={dotReference} />
        ) : (
          <div className="h-full w-full grid grid-cols-2 [&>*]:border-l [&>*]:border-slate-600">
            <div className="text-white px-1 py-2">Graph</div>
            <div className="text-white px-1 py-2">Reference</div>
            <div className="w-full h-full">
              {dotGraph?.type == "Graph" && (
                <Network dot={dotGraph.content.dot} />
              )}
            </div>
            <div className="w-full h-full">
              {dotReference && <Network dot={dotReference} />}
            </div>
          </div>
        )}
      </div>
      <Env
        compilationStatus={compilationStatus ?? null}
        env={env}
        src={src}
        input={input}
        setInput={setInput}
      />
      <Toaster />
    </div>
  );
};

type RightTab = "reference" | "stdout" | "stderr" | "validation";
const RIGHT_TABS_LABEL = {
  reference: "Reference output",
  stdout: "Raw output",
  stderr: "Debug output",
  validation: "Validation result",
} satisfies Record<RightTab, string>;

type EnvProps = {
  compilationStatus: CompilationStatus | null;
  env: Analysis;
  src: string;
  input: Input | null;
  setInput: (input: Input | null) => void;
};
const Env = ({ compilationStatus, env, src, input, setInput }: EnvProps) => {
  const { data: output } = useQuery(
    ["runAnalysis", src, input],
    ({}) => {
      if (!input) return void 0;

      return core.run_analysis(src, input);
    },
    {
      keepPreviousData: true,
    }
  );

  const referenceOutput = output?.markdown ?? "";

  const [tab, setTab] = useState<RightTab>("reference");
  const [inFlight, setInFlight] = useState(false);
  const [response, setResponse] = useState<null | AnalysisResponse>(null);

  useEffect(() => {
    if (input || !inputted.input) return;
    const json = inputted.input;

    const run = async () => {
      try {
        const fullInput = await core.complete_input_from_json(env, json);
        setInput(fullInput);
      } catch (e) {
        console.error(e);
      }
    };

    run();
  }, [env, input]);

  useEffect(() => {
    if (
      !input ||
      !compilationStatus ||
      compilationStatus.state.type != "Compiled"
    )
      return;

    if (input.analysis != env) {
      console.info(
        `not analyzing, since input was generated for ${input.analysis}, while current is ${env}`
      );
      return;
    }

    setInFlight(true);

    const controller = new AbortController();
    const promise = api.analyze(controller.signal, {
      analysis: env,
      input: input.json,
      src,
    });

    promise
      .then((res) => {
        setInFlight(false);
        setResponse(res);
      })
      .catch((e) => {
        if (e.name != "AbortError") console.error("analysis error:", e);
      });

    return () => controller.abort();
  }, [compilationStatus, src, input]);

  useEffect(() => {
    if (
      (inputted.input ? input : true) &&
      (input ? input.analysis != env : true)
    ) {
      const run = async () => {
        try {
          const input = await core.generate_input_for(src, env);
          setInput(input ?? null);
        } catch (e) {
          console.error(e);
        }
      };

      run();
    }
  }, [env, compilationStatus, input, src]);

  const indicatorState = computeIndicatorState({
    inFlight,
    compilationStatus,
    response,
  });

  const generateInput = useMutation(
    ({ src, env }: { src: string; env: Analysis }) =>
      core.generate_input_for(src, env),
    {
      onSuccess: (data) => {
        setInput(data ?? null);
      },
    }
  );

  return (
    <>
      <div className="grid place-items-start border-t border-slate-500 bg-slate-800">
        <div className="prose prose-invert px-4 pt-3">
          <ReactMarkdown
            children={input?.markdown ?? ""}
            remarkPlugins={[remarkGfm]}
          />
        </div>
        <button
          className="p-1.5 hover:bg-slate-600 transition text-slate-100 rounded-tr"
          onClick={() => generateInput.mutate({ src, env })}
        >
          <ArrowPathRoundedSquareIcon className="w-3" />
        </button>
      </div>
      <div
        className={
          "relative col-span-full w-full border-t-4 border-current transition " +
          INDICATOR_TEXT_COLOR[indicatorState]
        }
      >
        <div className="absolute right-0 top-0 z-10 -translate-y-full">
          <Indicator state={indicatorState} />
        </div>
      </div>
      {compilationStatus?.state.type == "CompileError" ? (
        <div
          className={
            "relative col-span-2 grid grid-cols-2 transition duration-700 " +
            (inFlight ? "blur-sm delay-100" : "")
          }
        >
          <div className="w-full space-y-2 px-4 py-2">
            <h3 className="text-lg font-bold italic text-white">
              Compilation Error
            </h3>
            <div className="prose prose-invert w-full max-w-none prose-pre:whitespace-pre-wrap prose-table:w-full">
              <pre className="whitespace-pre-wrap">
                <AnsiSpans code={compilationStatus.state.content.stdout} />
              </pre>
            </div>
          </div>
          <div className="w-full space-y-2 px-4 py-2">
            <h3 className="text-lg font-bold italic text-transparent">Error</h3>
            <div className="prose prose-invert w-full max-w-none prose-pre:whitespace-pre-wrap prose-table:w-full">
              <pre className="whitespace-pre-wrap">
                <AnsiSpans code={compilationStatus.state.content.stderr} />
              </pre>
            </div>
          </div>
        </div>
      ) : response ? (
        <div
          className={
            "relative col-span-2 grid grid-cols-2 transition duration-700 " +
            (inFlight ? "blur-sm delay-100" : "")
          }
        >
          {/* <div className="absolute inset-0 grid grid-cols-[1fr_2fr] divide-slate-600 overflow-y-auto"> */}
          <div className="absolute inset-0 grid grid-cols-2 justify-center divide-slate-600 overflow-y-auto bg-slate-800">
            {response.validation_result ? (
              <div className="flex w-full max-w-prose flex-col space-y-2 bg-slate-800 px-4 py-2 text-xl text-white">
                {response.validation_result.type == "InvalidOutput" ? (
                  <>
                    <h3 className="text-lg">Invalid Output</h3>
                    <div className="prose prose-invert w-full max-w-none prose-table:w-full">
                      <ReactMarkdown
                        children={[
                          "Failed to parse output. Make sure that it is valid JSON, and it has the correct structure.",

                          "**Error:**",
                          "```\n" +
                            (response.validation_result.content.error ?? "") +
                            "\n```",

                          "**Your output:**",

                          "```\n" +
                            response.validation_result.content.output.trim() +
                            "\n```",

                          ...(response.validation_result.content
                            .expected_output_format
                            ? [
                                "**Expected output format:**",

                                "```\n" +
                                  response.validation_result.content.expected_output_format.trim() +
                                  "\n```",
                              ]
                            : []),
                        ].join("\n\n")}
                        remarkPlugins={[remarkGfm]}
                      />
                    </div>
                  </>
                ) : response.validation_result.type == "InvalidInput" ? (
                  <>
                    <h3 className="text-lg">Invalid Input</h3>
                    <div className="prose prose-invert w-full max-w-none prose-table:w-full">
                      <ReactMarkdown
                        children={[
                          "The input was invalid. Consider regenerating it.",

                          "**Error:**",
                          "```\n" +
                            (response.validation_result.content.error ?? "") +
                            "\n```",

                          "**Given input:**",

                          "```\n" +
                            response.validation_result.content.input.trim() +
                            "\n```",
                        ].join("\n\n")}
                        remarkPlugins={[remarkGfm]}
                      />
                    </div>
                  </>
                ) : (
                  <>
                    <h3 className="text-lg">Output</h3>
                    <div className="prose prose-invert w-full max-w-none prose-table:w-full">
                      <ReactMarkdown
                        children={response.parsed_markdown ?? ""}
                        remarkPlugins={[remarkGfm]}
                      />
                    </div>
                  </>
                )}
              </div>
            ) : (
              <div className="flux w-full space-y-2 px-4 py-2">
                <h3 className="text-lg font-bold italic text-white">Error</h3>
                <div
                  className="prose prose-invert w-full max-w-none prose-pre:whitespace-pre-wrap prose-table:w-full"
                  title={JSON.stringify(response.stderr.trim())}
                >
                  <pre className="whitespace-pre-wrap">
                    <AnsiSpans code={response.stderr} />
                  </pre>
                </div>
              </div>
            )}
            <div className="flex w-full max-w-prose flex-col space-y-2 bg-slate-800 px-4 py-2 text-xl text-white">
              <select
                className="flex appearance-none bg-transparent text-lg"
                value={tab}
                onChange={(e) => setTab(e.target.value as RightTab)}
              >
                {Object.entries(RIGHT_TABS_LABEL).map(([value, label]) => (
                  <option key={value} value={value}>
                    {label}
                  </option>
                ))}
              </select>
              <div className="prose prose-invert w-full max-w-none prose-table:w-full">
                {tab == "reference" ? (
                  <ReactMarkdown
                    children={referenceOutput}
                    remarkPlugins={[remarkGfm]}
                  />
                ) : tab == "stderr" ? (
                  <pre className="whitespace-pre-wrap">
                    <AnsiSpans code={response.stderr} />
                  </pre>
                ) : tab == "stdout" ? (
                  <pre className="whitespace-pre-wrap">
                    <AnsiSpans code={response.stdout} />
                  </pre>
                ) : tab == "validation" ? (
                  <pre className="whitespace-pre-wrap">
                    {response.validation_result
                      ? JSON.stringify(response.validation_result, null, 2)
                      : ""}
                  </pre>
                ) : null}
              </div>
            </div>
          </div>
        </div>
      ) : (
        <div className="col-span-2 grid place-items-center text-4xl">
          <div className="animate-bounce">👻</div>
        </div>
      )}
    </>
  );
};

const computeIndicatorState = ({
  inFlight,
  compilationStatus,
  response,
}: {
  inFlight: boolean;
  compilationStatus: CompilationStatus | null;
  response: AnalysisResponse | null;
}) => {
  if (inFlight || !compilationStatus) return IndicatorState.Working;

  if (compilationStatus.state.type == "CompileError")
    return IndicatorState.Error;

  if (compilationStatus.state.type != "Compiled") return IndicatorState.Working;

  if (!response || !response.validation_result) return IndicatorState.Error;

  if (
    response.validation_result.type == "CorrectTerminated" ||
    response.validation_result.type == "CorrectNonTerminated"
  )
    return IndicatorState.Correct;

  if (response.validation_result.type == "Mismatch")
    return IndicatorState.Mismatch;

  if (response.validation_result.type == "InvalidOutput")
    return IndicatorState.Mismatch;

  if (response.validation_result.type == "InvalidInput")
    return IndicatorState.Mismatch;

  if (response.validation_result.type == "TimeOut")
    return IndicatorState.TimeOut;

  return IndicatorState.Working;
};

const PredicateEvaluation = ({ predicate }: { predicate: string }) => {
  useEffect(() => {
    const run = async () => {
      const result = await import("../lib/z3");
      console.log({ result });
    };

    run();
  });

  return <code>{predicate} ✅❌⏳</code>;
};

const AnsiSpans = ({ code }: { code: string }) => (
  <>
    {parse(code).spans.map((s) => {
      const colors: Record<string, string> = {
        black: "rgb(0,0,0)",
        darkGray: "rgb(100,100,100)",
        lightGray: "rgb(200,200,200)",
        white: "rgb(255,255,255)",

        red: "rgb(204,0,0)",
        lightRed: "rgb(255,51,0)",

        green: "rgb(0,204,0)",
        lightGreen: "rgb(51,204,51)",

        yellow: "rgb(204,102,0)",
        lightYellow: "rgb(255,153,51)",

        blue: "rgb(0,0,255)",
        lightBlue: "rgb(26,140,255)",

        magenta: "rgb(204,0,204)",
        lightMagenta: "rgb(255,0,255)",

        cyan: "rgb(0,153,255)",
        lightCyan: "rgb(0,204,255)",
      };

      const values: React.CSSProperties = {};

      if (s.bold) values.fontWeight = "bold";
      if (s.color?.name)
        values.color =
          s.color.name in colors ? colors[s.color.name] : s.color.name;
      if (s.italic) values.fontStyle = "italic";

      return (
        <span key={s.text} style={values}>
          {s.text}
        </span>
      );
    })}
  </>
);
export const Network = React.memo(({ dot }: { dot: string }) => {
  const [container, setContainer] = React.useState<null | HTMLDivElement>();

  React.useEffect(() => {
    if (!container) return;

    const run = async () => {
      const visPromise = import("vis-network/esnext");
      const vis = await visPromise;

      const data = vis.parseDOTNetwork(dot);

      new vis.Network(container, data, {
        interaction: { zoomView: false },
        nodes: {
          color: {
            background: "#666666",
            border: "#8080a0",
            highlight: "#80a0ff",
          },
          font: {
            color: "white",
          },
          borderWidth: 1,
          shape: "circle",
          size: 30,
        },
        edges: {
          color: "#D0D0FF",
          font: {
            color: "white",
            strokeColor: "#200020",
          },
        },
        autoResize: true,
      });
    };

    const debounce = requestAnimationFrame(() => run().catch(console.error));
    return () => cancelAnimationFrame(debounce);
  }, [container, dot]);

  return <div className="h-full w-full" ref={setContainer}></div>;
});

import React from "react";
import { useEffect, useState } from "react";
import { WebApplication } from "verification-lawyer";
import { StretchEditor } from "./StretchEditor";
import { ArrowPathRoundedSquareIcon } from "@heroicons/react/24/solid";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import "./z3";

const app = WebApplication.new();

export const App = () => {
  return <AppA />;
};

const ENVS = ["Sign", "Step-wise", "Security", "Program Verification"] as const;
type Envs = (typeof ENVS)[number];

const AppA = () => {
  const [src, setSrc] = useState(app.generate_program());
  const [deterministic, setDeterministic] = useState(true);
  const [env, setEnv] = useState<Envs>("Step-wise");
  const [dot, setDot] = useState<null | string>(null);

  useEffect(() => {
    setDot(app.dot(deterministic, src));
  }, [deterministic, src]);

  return (
    <div className="grid h-screen grid-rows-[auto_1fr]">
      <nav className="bg-slate-900 font-bold text-slate-200">
        <a className="flex p-2 text-lg">Verification Lawyer</a>
      </nav>
      <div className="grid min-h-0 grid-cols-[1fr_2fr] grid-rows-[1fr_auto_1fr] divide-slate-600">
        <div className="grid grid-rows-[auto_1fr] divide-y divide-slate-600">
          <div className="grid grid-cols-3 divide-x divide-slate-600 border-r border-slate-600">
            <button
              className="flex items-center justify-center space-x-1 bg-slate-800 py-1 px-1.5 text-sm text-white transition hover:bg-slate-700 active:bg-slate-900"
              onClick={() => {
                setSrc(app.generate_program());
              }}
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
              onChange={(e) => setEnv(e.target.value as Envs)}
            >
              {ENVS.map((e) => (
                <option key={e}>{e}</option>
              ))}
            </select>
          </div>
          <div className="relative bg-slate-200">
            <StretchEditor source={src} onChange={setSrc} />
          </div>
        </div>
        <div className="relative row-span-2 bg-slate-800">
          {dot && <Network dot={dot} />}
        </div>
        <Env env={env} src={src} />
      </div>
    </div>
  );
};

const Env = ({ env, src }: { env: Envs; src: string }) => {
  const [[inputJson, input, output], setIO] = useState(["", "", ""]);
  const [remoteOutput, setRemoteOutput] = useState("");

  useEffect(() => {
    const headers = new Headers();
    headers.append("Content-Type", "application/json");

    const body = JSON.stringify({
      analysis: {
        Sign: "sign",
        "Step-wise": "interpreter",
        Security: "security",
        "Program Verification": "pv",
      }[env],
      src,
      input: inputJson,
      // '{"determinism":{"Case":"Deterministic"}}',
    });

    fetch("http://localhost:3000/analyze", { method: "POST", headers, body })
      .then((res) => res.json())
      .then((result) => {
        console.log(result);
        setRemoteOutput(result.parsed_markdown);
      })
      .catch((error) => console.log("error", error));
  }, [src, inputJson]);

  useEffect(() => {
    try {
      switch (env) {
        case "Security":
          setIO(JSON.parse(app.security(src)));
          break;
        case "Sign":
          setIO(JSON.parse(app.sign(src)));
          break;
        case "Step-wise":
          setIO(JSON.parse(app.step_wise(src)));
          break;
        case "Program Verification":
          setIO(JSON.parse(app.pv(src)));
          break;
      }
    } catch (e) {
      console.error(e);
    }
  }, [env, src]);

  return (
    <>
      <div className="grid place-items-start border-y border-slate-500 bg-slate-800 px-4 py-3 text-xl">
        <div className="prose prose-invert">
          <ReactMarkdown children={input} remarkPlugins={[remarkGfm]} />
        </div>
      </div>
      <div className="relative col-span-2 grid">
        {/* <div className="absolute inset-0 grid grid-cols-[1fr_2fr] divide-slate-600 overflow-y-auto"> */}
        <div className="absolute inset-0 flex justify-center divide-slate-600 overflow-y-auto bg-slate-800">
          <div className="flex w-full max-w-prose flex-col space-y-2 bg-slate-800 px-4 py-2 text-xl text-white">
            <h3 className="text-lg">Output 1</h3>
            <div className="prose prose-invert w-full max-w-none prose-table:w-full">
              <ReactMarkdown children={output} remarkPlugins={[remarkGfm]} />
            </div>
          </div>
          <div className="flex w-full max-w-prose flex-col space-y-2 bg-slate-800 px-4 py-2 text-xl text-white">
            <h3 className="text-lg">Output 2</h3>
            <div className="prose prose-invert w-full max-w-none prose-table:w-full">
              <ReactMarkdown
                children={remoteOutput}
                remarkPlugins={[remarkGfm]}
              />
            </div>
          </div>
        </div>
      </div>
    </>
  );
};

export const Network = React.memo(({ dot }: { dot: string }) => {
  const [container, setContainer] = React.useState<null | HTMLDivElement>();

  React.useEffect(() => {
    if (!container) return;

    const run = async () => {
      const visPromise = import("vis-network/esnext");
      const vis = await visPromise;

      const data = vis.parseDOTNetwork(dot);

      const network = new vis.Network(container, data, {
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

    // const debounce = requestIdleCallback(() => run().catch(console.error));
    // return () => cancelIdleCallback(debounce);
  }, [container, dot]);

  return <div className="h-full w-full" ref={setContainer}></div>;
});

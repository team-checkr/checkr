import React, { useRef } from "react";
import { useEffect, useState } from "react";
import { WebApplication, WasmZ3 } from "verification-lawyer";
import { StretchEditor } from "./StretchEditor";
import {
  ArrowPathRoundedSquareIcon,
  ChevronDownIcon,
} from "@heroicons/react/24/solid";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import "./z3";

const app = WebApplication.new();

export const App = () => {
  return <AppA />;
};

type Envs = "Sign" | "Step-wise" | "Security";

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
      <div className="grid grid-cols-[1fr_2fr] grid-rows-[1fr_auto_1fr] divide-slate-600">
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
              className="flex items-center justify-center space-x-1 bg-slate-800 py-1 px-1.5 text-center text-sm text-white transition hover:bg-slate-700 active:bg-slate-900"
              value={env}
              onChange={(e) => setEnv(e.target.value as Envs)}
            >
              <option>Step-wise</option>
              <option>Sign</option>
              <option>Security</option>
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
  const [[input, output], setIO] = useState(["", ""]);

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
        <div className="absolute inset-0 flex justify-evenly divide-slate-600 overflow-y-auto bg-slate-800">
          <div className="flex w-full max-w-prose flex-col space-y-2 bg-slate-800 px-4 py-2 text-xl text-white">
            <h3 className="text-lg">Output 1</h3>
            <div className="prose prose-invert">
              <ReactMarkdown children={output} remarkPlugins={[remarkGfm]} />
            </div>
          </div>
          <div className="flex w-full max-w-prose flex-col space-y-2 bg-slate-800 px-4 py-2 text-xl text-white">
            <h3 className="text-lg">Output 2</h3>
            <div className="prose prose-invert">
              <ReactMarkdown children={output} remarkPlugins={[remarkGfm]} />
            </div>
          </div>
        </div>
      </div>
    </>
  );
};

const Old = () => {
  const [app, setApp] = useState<WebApplication | null>(null);

  const [envs, setEnvs] = useState<null | {
    program: string;
    dot: string;
    envs: [string, [string, string]][];
  }>(null);

  useEffect(() => {
    (async () => {
      const z3 = await WasmZ3.new();
      await z3.run();
    })();

    const app = WebApplication.new();
    setApp(app);
    setEnvs(JSON.parse(app.generate()));

    // const int = setInterval(() => {
    //   setEnvs(JSON.parse(app.generate()));
    // }, 10);

    // return () => clearInterval(int);
  }, []);

  const interval = useRef(0);

  return (
    <div
      className="grid h-screen w-full grid-rows-[60vh_40vh] text-zinc-50"
      style={{ background: "#200020" }}
    >
      <div className="flex flex-col">
        <button
          className="border border-zinc-500 p-2 text-sm transition active:bg-violet-900/20"
          onMouseDown={() => {
            clearInterval(interval.current);
            interval.current = setInterval(() => {
              if (app) setEnvs(JSON.parse(app.generate()));
            }, 10);
          }}
          onMouseUp={() => {
            clearInterval(interval.current);
          }}
        >
          Generate program
        </button>
        <div className="grid flex-1 shrink grid-cols-2 overflow-hidden">
          <div className="relative h-full">
            <StretchEditor source={envs?.program ?? ""} />
          </div>
          {/* <pre className="overflow-y-auto">{envs?.program}</pre> */}
          <div>{envs && <Network dot={envs.dot} />}</div>
        </div>
      </div>
      {/* <div className="grid grid-cols-3 grid-rows-1 gap-2"> */}
      <div className="grid grid-cols-1 grid-rows-1 gap-2">
        {envs?.envs.slice(1, 2).map(([name, [input, output]]) => (
          <div className="relative grid grid-rows-[auto_1fr]" key={name}>
            <h1 className="mb-2 border-b border-zinc-600 text-2xl">{name}</h1>
            <div className="grid grid-cols-2">
              <div className="grid grid-rows-[auto_1fr]">
                <h2 className="mb-2 border-b border-zinc-600 text-lg font-bold">
                  Input
                </h2>
                <div className="relative">
                  <div className="prose prose-invert absolute inset-0 overflow-y-auto text-xs">
                    {/* {input} */}
                    <ReactMarkdown
                      children={input}
                      remarkPlugins={[remarkGfm]}
                    />
                  </div>
                </div>
              </div>
              <div className="grid grid-rows-[auto_1fr]">
                <h2 className="mb-2 border-b border-zinc-600 text-lg font-bold">
                  Output
                </h2>
                <div className="relative">
                  <div className="prose prose-invert absolute inset-0 overflow-y-auto text-xs">
                    {/* {output} */}
                    <ReactMarkdown
                      children={output}
                      remarkPlugins={[remarkGfm]}
                    />
                  </div>
                </div>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
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

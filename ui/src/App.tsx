import React, { useRef } from "react";
import { useEffect, useState } from "react";
import { WebApplication, WasmZ3 } from "verification-lawyer";
import { StretchEditor } from "./StretchEditor";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import "./z3";

export const App = () => {
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
    try {
      setEnvs(JSON.parse(app.generate()));
    } catch (e) {
      setEnvs(JSON.parse(app.generate()));
    }

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
              setEnvs(JSON.parse(app.generate()));
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
      <div className="grid grid-cols-3 grid-rows-1 gap-2">
        {envs?.envs.map(([name, [input, output]]) => (
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

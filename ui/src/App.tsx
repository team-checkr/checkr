import { useEffect, useState } from "react";
import { WebApplication } from "verification-lawyer";

export const App = () => {
  const [app, setApp] = useState<WebApplication | null>(null);

  const [envs, setEnvs] = useState<null | {
    program: string;
    envs: [string, [unknown, unknown]][];
  }>(null);

  useEffect(() => {
    const app = WebApplication.new();
    setApp(app);
    setEnvs(JSON.parse(app.generate()));

    // const int = setInterval(() => {
    //   setEnvs(JSON.parse(app.generate()));
    // }, 10);

    // return () => clearInterval(int);
  }, []);

  return (
    <div className="grid min-h-screen w-full place-items-center">
      <div>
        <button
          className="rounded border p-4 text-xl shadow"
          onClick={() => {
            setEnvs(JSON.parse(app.generate()));
          }}
        >
          Generate program
        </button>
        <pre className="h-[40vh] overflow-y-auto">{envs?.program}</pre>
        <div className="grid grid-cols-3 gap-2">
          {envs?.envs.map(([name, [input, output]]) => (
            <div className="">
              <h1 className="mb-2 border-b text-2xl">{name}</h1>
              <div className="grid grid-cols-2">
                <div>
                  <h2 className="mb-2 border-b text-lg font-bold">Input</h2>
                  <pre className="h-[40vh] overflow-y-auto">
                    {JSON.stringify(input, null, 2)}
                  </pre>
                </div>
                <div>
                  <h2 className="mb-2 border-b text-lg font-bold">Output</h2>
                  <pre className="h-[40vh] overflow-y-auto">
                    {JSON.stringify(output, null, 2)}
                  </pre>
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};

import { useEffect, useState } from "react";
import {
  hello_wasm,
  generate_program,
  WebApplication,
} from "verification-lawyer";

export const App = () => {
  const [program, setProgram] = useState("");
  const [envs, setEnvs] = useState([] as string[]);

  useEffect(() => {
    const app = WebApplication.new();

    setEnvs(app.list_envs().split(","));

    setProgram(generate_program());

    // const int = setInterval(() => {
    //   setProgram(generate_program());
    // }, 100);

    // return () => clearInterval(int);
  }, []);

  return (
    <div className="grid min-h-screen w-full place-items-center">
      <div>
        <div className="text-5xl">{hello_wasm("Camulla")}</div>
        <ol>
          {envs.map((e) => (
            <li key={e}>{e}</li>
          ))}
        </ol>
        <button
          className="rounded border p-4 text-xl shadow"
          onClick={() => {
            setProgram(generate_program());
          }}
        >
          Generate program
        </button>
        <pre className="h-[70vh] w-96">{program}</pre>
      </div>
    </div>
  );
};

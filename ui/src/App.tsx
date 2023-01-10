import { hello_wasm } from "verification-lawyer";

export const App = () => {
  return (
    <div className="grid min-h-screen w-full place-items-center">
      <div className="text-5xl">{hello_wasm("Camulla")}</div>
    </div>
  );
};

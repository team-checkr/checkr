declare module "z3-solver/build/z3-built" {
  export default function (opts: {
    locateFile: (f: string) => f;
    mainScriptUrlOrBlob: string;
  }): any;
}

interface Window {
  __z3Init(): Promise<string>;
  __z3Run(ctx: String, cmd: string): Promise<string>;
}

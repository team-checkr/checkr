/// <reference path="z3-wrapper.d.ts" />

import { Mutex } from "async-mutex";
import { init, type Z3_context } from "z3-solver/build/low-level";
import initZ3 from "z3-solver/build/z3-built";

type Z3Instance = Awaited<ReturnType<typeof init>>;

const lock = new Mutex();
let stored = init(
  async () =>
    await initZ3({
      locateFile: (f) => f,
      mainScriptUrlOrBlob: "z3-built.js",
    })
);
const borrow = async <T>(f: (x: Z3Instance) => Promise<T>): Promise<T> => {
  const release = await lock.acquire();
  return f(await stored).finally(release);
};

const contexts = new Map<string, Z3_context>();
const initContext = () =>
  borrow(async ({ Z3 }) => {
    const n = contexts.size.toString();
    const cfg = Z3.mk_config();
    const ctx = Z3.mk_context(cfg);
    Z3.del_config(cfg);
    await Z3.eval_smtlib2_string(ctx, "(set-option :smtlib2_compliant true)");
    contexts.set(n, ctx);
    return n;
  });
const runInContext = async (ctx: string, cmd: string): Promise<string> =>
  borrow(async ({ Z3 }) => {
    const c = contexts.get(ctx)!;
    return Z3.eval_smtlib2_string(c, cmd);
  });

window.__z3Init = initContext;
window.__z3Run = runInContext;

import { Mutex } from 'async-mutex';
import { init } from 'z3-solver/build/low-level';
// @ts-ignore
import initZ3 from 'z3-solver/build/z3-built.js';

let stored = init(async () => {
  const files = {
    'z3-built.js': await import('z3-solver/build/z3-built?url'),
    'z3-built.wasm': await import('z3-solver/build/z3-built.wasm?url'),
    'z3-built.worker.js': await import('z3-solver/build/z3-built.worker?url'),
  };
  return initZ3({
    locateFile: (f: string) => {
      if (!(f in files)) throw new Error(`unknown z3 file: ${f}`);
      return files[f as keyof typeof files].default;
    },
    mainScriptUrlOrBlob: files['z3-built.js'].default,
  });
});

type Z3Instance = Awaited<ReturnType<typeof init>>;

const lock = new Mutex();
const borrow = async <T>(f: (x: Z3Instance) => Promise<T>): Promise<T> => {
  const release = await lock.acquire();
  return f(await stored).finally(release);
};

export type RunOptions = {
  prelude?: string;
  onStart?: () => void;
};

export const run = (
  query: string,
  options: RunOptions = {},
): { cancel: () => void; result: Promise<'cancelled' | string[]> } => {
  let isCancelled = false;
  const cancel = () => {
    isCancelled = true;
  };

  return {
    cancel,
    result: borrow(async ({ Z3 }) => {
      options.onStart?.();

      const timeout = 5000;

      Z3.global_param_set('timeout', String(timeout));

      const cfg = Z3.mk_config();
      const ctx = Z3.mk_context(cfg);
      Z3.del_config(cfg);

      console.group('smt');

      if (options.prelude) {
        console.info('prelude:', '\n' + options.prelude);
        await Z3.eval_smtlib2_string(ctx, options.prelude);
      }

      const results: string[] = [];

      for (const l of query.split('\n')) {
        if (isCancelled) {
          console.info('cancelled');
          break;
        }

        console.info('evaluating:', l);

        const timeStart = new Date().getTime();
        const res = await Z3.eval_smtlib2_string(ctx, l);
        const timeEnd = new Date().getTime();
        if (timeEnd - timeStart >= timeout) {
          console.info('timeout');
          results.push('timeout');
        } else {
          console.info('    result:', res);
          results.push(res);
        }
      }

      const version = await Z3.eval_smtlib2_string(ctx, '(get-info :version)');
      console.info(version);
      const stats = await Z3.eval_smtlib2_string(ctx, '(get-info :all-statistics)');
      console.info(stats);

      console.groupEnd();

      Z3.del_context(ctx);

      if (isCancelled) return 'cancelled';

      return results;
    }),
  };
};

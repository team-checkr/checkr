import { Mutex } from 'async-mutex';
import { init } from 'z3-solver/build/low-level';
import initZ3 from 'z3-solver/build/z3-built';

let stored = init(async () => {
	const files = {
		'z3-built.js': await import('z3-solver/build/z3-built?url'),
		'z3-built.wasm': await import('z3-solver/build/z3-built.wasm?url'),
		'z3-built.worker.js': await import('z3-solver/build/z3-built.worker?url')
	};
	return initZ3({
		locateFile: (f) => {
			if (!(f in files)) throw new Error(`unknown z3 file: ${f}`);
			return files[f as keyof typeof files].default;
		},
		mainScriptUrlOrBlob: files['z3-built.js'].default
	});
});

type Z3Instance = Awaited<ReturnType<typeof init>>;

const lock = new Mutex();
const borrow = async <T>(f: (x: Z3Instance) => Promise<T>): Promise<T> => {
	const release = await lock.acquire();
	return f(await stored).finally(release);
};

export const run = async (query: string, onStart?: () => void) =>
	borrow(async ({ Z3 }) => {
		onStart?.();

		const timeout = 1000;

		Z3.global_param_set('timeout', String(timeout));

		// TODO: Add a timeout of like 30 seconds
		const cfg = Z3.mk_config();
		const ctx = Z3.mk_context(cfg);
		Z3.del_config(cfg);

		const results: string[] = [];

		for (const l of query.split('\n')) {
			console.info("evaluating", l);
			const timeStart =new Date().getTime();
			const res = await Z3.eval_smtlib2_string(ctx, l);
			const timeEnd = new Date().getTime();
			if (timeEnd - timeStart >= timeout) {

				results.push('timeout');
			} else {

				results.push(res);
			}
		}

		Z3.del_context(ctx);

		return results;
	});

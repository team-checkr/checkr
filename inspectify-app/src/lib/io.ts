import { derived, writable, type Writable } from 'svelte/store';
import { ce_shell, api, driver, type ce_core } from './api';
import { compilationStatusStore } from './events';
import { selectedJobId } from './jobs';

type Mapping = { [A in ce_shell.Analysis]: (ce_shell.Envs & { analysis: A })['io'] };

type OutputState = 'None' | 'Stale' | 'Current';

export type Input<A extends ce_shell.Analysis> = Mapping[A]['input'];
export type Output<A extends ce_shell.Analysis> = Mapping[A]['output'];

export type Io<A extends ce_shell.Analysis> = {
	input: Writable<Input<A>>;
	output: Writable<Output<A>>;
	referenceOutput: Writable<Output<A>>;
	outputState: Writable<OutputState>;
	validation: Writable<ce_core.ValidationResult | { type: 'Failure'; message: string } | null>;
	generate: () => Promise<Input<A>>;
	latestJobId: Writable<driver.job.JobId | null>;
};

const ios: Partial<Record<ce_shell.Analysis, Io<ce_shell.Analysis>>> = {};

const initializeIo = <A extends ce_shell.Analysis>(
	analysis: A,
	defaultInput: Input<A>,
	defaultOutput: Output<A>
): Io<A> => {
	const input = writable<Input<A>>(defaultInput);
	const output = writable<Output<A>>(defaultOutput);
	const referenceOutput = writable<Output<A>>(defaultOutput);
	const outputState = writable<OutputState>('None');
	const validation = writable<
		ce_core.ValidationResult | { type: 'Failure'; message: string } | null
	>(null);
	const latestJobId = writable<driver.job.JobId | null>(null);

	let activeJob: driver.job.JobId | null = null;
	let activeRequest: { abort: () => void } | null = null;

	const [debounceAnalysis] = debounce(
		(input: Input<A>) => api.analysis({ analysis, json: input }),
		200
	);

	derived([compilationStatusStore, input], (x) => x).subscribe(
		async ([compilationStatus, input]) => {
			if (!compilationStatus || !input) return;
			if (compilationStatus.state != 'Succeeded') return;

			if (activeJob) {
				api.jobsCancel(activeJob);
				activeJob = null;
			}
			if (activeRequest) {
				activeRequest.abort();
				activeRequest = null;
			}

			outputState.set('Stale');

			const request = await debounceAnalysis(input);
			const id = await request.data;
			activeJob = id;
			latestJobId.set(id);
			selectedJobId.set(id);
			const innerRequest = api.jobsWait(id);
			activeRequest = innerRequest;
			innerRequest.data.then((result) => {
				if (innerRequest == activeRequest) activeRequest = null;
				else return;
				if (activeJob == id) activeJob = null;
				else return;
				if (result.kind == 'AnalysisSuccess') {
					const { data } = result;
					output.set(data.output.json as any);
					referenceOutput.set(data.reference_output.json as any);
					validation.set(data.validation as any);
					outputState.set('Current');
				} else if (result.kind == 'Failure') {
					outputState.set('Current');
					validation.set({ type: 'Failure', message: result.data.error });
				}
			});
		}
	);

	const generate = () =>
		api.generate({ analysis }).data.then((result) => {
			input.set(result.json as any);
			return result.json as any;
		});

	generate();

	return {
		input,
		output,
		referenceOutput,
		outputState,
		validation,
		generate,
		latestJobId
	};
};

export const useIo = <A extends ce_shell.Analysis>(
	analysis: A,
	defaultInput: Input<A>,
	defaultOutput: Output<A>
): Io<A> => {
	if (!ios[analysis]) {
		ios[analysis] = initializeIo(analysis, defaultInput, defaultOutput);
	}
	return ios[analysis] as Io<A>;
};

function debounce<A = unknown, R = void>(
	fn: (args: A) => R,
	ms: number
): [(args: A) => Promise<R>, () => void] {
	let timer: number;

	const debouncedFunc = (args: A): Promise<R> =>
		new Promise((resolve) => {
			if (timer) {
				clearTimeout(timer);
			}

			timer = setTimeout(() => {
				resolve(fn(args));
			}, ms);
		});

	const teardown = () => clearTimeout(timer);

	return [debouncedFunc, teardown];
}

import { derived, writable, type Writable } from 'svelte/store';
import { ce_shell, api, driver, type ce_core } from './api';
import { compilationStatusStore } from './events';

type Mapping = { [A in ce_shell.Analysis]: (ce_shell.Envs & { analysis: A })['io'] };

type OutputState = 'None' | 'Stale' | 'Current';

export type Input<A extends ce_shell.Analysis> = Mapping[A]['input'];
export type Output<A extends ce_shell.Analysis> = Mapping[A]['output'];

export type Io<A extends ce_shell.Analysis> = {
	input: Writable<Input<A>>;
	output: Writable<Output<A>>;
	outputState: Writable<OutputState>;
	validation: Writable<ce_core.ValidationResult | null>;
	generate: () => Promise<Input<A>>;
};

const ios: Partial<Record<ce_shell.Analysis, Io<ce_shell.Analysis>>> = {};

const initializeIo = <A extends ce_shell.Analysis>(
	analysis: A,
	defaultInput: Input<A>,
	defaultOutput: Output<A>
): Io<A> => {
	const input = writable<Input<A>>(defaultInput);
	const output = writable<Output<A>>(defaultOutput);
	const outputState = writable<OutputState>('None');
	const validation = writable<ce_core.ValidationResult | null>(null);

	let activeJob: null | driver.job.JobId = null;
	let activeRequest: null | { abort: () => void } = null;

	derived([compilationStatusStore, input], (x) => x).subscribe(([compilationStatus, input]) => {
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
		const request = api.analysis({ analysis, json: input });
		request.data.then((id) => {
			activeJob = id;
			const innerRequest = api.jobsWait(id);
			innerRequest.data.then((result) => {
				if (innerRequest == activeRequest) activeRequest = null;
				if (activeJob == id) activeJob = null;
				if (result) {
					output.set(result.output.json as any);
					validation.set(result.validation as any);
					outputState.set('Current');
				}
			});
			activeRequest = innerRequest;
		});
	});

	const generate = () =>
		api.generate({ analysis }).data.then((result) => {
			input.set(result.json as any);
			return result.json as any;
		});

	generate();

	return {
		input,
		output,
		outputState,
		validation,
		generate
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

// Object.fromEntries(
// 	ce_shell.ANALYSIS.map((a) => {
// 		const input = writable<Mapping[ce_shell.Analysis]['input'] | null>(null);
// 		const output = writable<Mapping[ce_shell.Analysis]['output'] | null>(null);
// 		const validation = writable<ce_core.ValidationResult | null>(null);

// 		let activeJob: null | driver.job.JobId = null;
// 		let activeRequest: null | { abort: () => void } = null;

// 		derived([compilationStatusStore, input], (x) => x).subscribe(([compilationStatus, input]) => {
// 			if (!compilationStatus || !input) return;
// 			if (compilationStatus.state != 'Succeeded') return;

// 			if (activeJob) {
// 				api.jobsCancel(activeJob);
// 				activeJob = null;
// 			}
// 			if (activeRequest) {
// 				activeRequest.abort();
// 				activeRequest = null;
// 			}

// 			const request = api.analysis({ analysis: a, json: input });
// 			request.data.then((id) => {
// 				activeJob = id;
// 				const innerRequest = api.jobsWait(id);
// 				innerRequest.data.then((result) => {
// 					if (innerRequest == activeRequest) activeRequest = null;
// 					if (activeJob == id) activeJob = null;
// 					if (result) {
// 						output.set(result.output.json as any);
// 						validation.set(result.validation as any);
// 					}
// 				});
// 				activeRequest = innerRequest;
// 			});
// 		});

// 		const generate = () =>
// 			api.generate({ analysis: a }).data.then((result) => {
// 				input.set(result.json as any);
// 				return result.json as any;
// 			});

// 		generate();

// 		return [
// 			a,
// 			{
// 				input,
// 				output,
// 				outputState: 'None',
// 				validation,
// 				generate
// 			} satisfies Io<typeof a>
// 		];
// 	})
// ) as any;

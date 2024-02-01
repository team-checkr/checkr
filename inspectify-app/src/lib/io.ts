import { derived, writable, type Writable } from 'svelte/store';
import { ce_shell, api, driver, type ce_core } from './api';
import { compilationStatusStore } from './jobs';

type Mapping = { [A in ce_shell.Analysis]: (ce_shell.Envs & { analysis: A })['io'] };

type OutputState = 'None' | 'Stale' | 'Current';

export type Io<A extends ce_shell.Analysis> = {
	input: Writable<Mapping[A]['input'] | null>;
	output: Writable<Mapping[A]['output'] | null>;
	outputState: OutputState;
	validation: Writable<ce_core.ValidationResult | null>;
	generate: () => Promise<Mapping[A]['input']>;
};

const ios: Record<ce_shell.Analysis, Io<ce_shell.Analysis>> = Object.fromEntries(
	ce_shell.ANALYSIS.map((a) => {
		const input = writable(null);
		const output = writable(null);
		const validation = writable(null);

		let activeJob: null | driver.job.JobId = null;

		derived([compilationStatusStore, input], ([compilationStatus, input]) => [
			compilationStatus,
			input
		]).subscribe(([compilationStatus, input]) => {
			if (!compilationStatus || !input) return;
			if (compilationStatus.state != 'Succeeded') return;

			if (activeJob) {
				api.cancelJob(activeJob);
				activeJob = null;
			}

			api.analysis({ analysis: a, json: input }).then((id) => {
				activeJob = id;
				api.waitForJob(id).then((result) => {
					if (result) {
						output.set(result.output.json as any);
						validation.set(result.validation as any);
					}
				});
			});
		});

		return [
			a,
			{
				input,
				output,
				outputState: 'None',
				validation,
				generate: () =>
					api.generate({ analysis: a }).then((result) => {
						input.set(result.json as any);
						return result.json as any;
					})
			} satisfies Io<typeof a>
		];
	})
) as any;

export const useIo = <A extends ce_shell.Analysis>(analysis: A): Io<A> => ios[analysis] as Io<A>;

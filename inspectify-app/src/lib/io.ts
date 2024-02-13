import { derived, writable, type Writable } from 'svelte/store';
import { ce_shell, api, driver, type ce_core } from './api';
import { compilationStatusStore } from './events';
import { selectedJobId } from './jobs';
import { browser } from '$app/environment';

type Mapping = { [A in ce_shell.Analysis]: (ce_shell.Envs & { analysis: A })['io'] };

type OutputState = 'None' | 'Stale' | 'Current';

export type Input<A extends ce_shell.Analysis> = Mapping[A]['input'];
export type Output<A extends ce_shell.Analysis> = Mapping[A]['output'];

export type Results<A extends ce_shell.Analysis> = {
  outputState: OutputState;
  output: Output<A>;
  referenceOutput: Output<A>;
  validation: ce_core.ValidationResult | { type: 'Failure'; message: string } | null;
  latestJobId: driver.job.JobId | null;
};

export type Io<A extends ce_shell.Analysis> = {
  input: Writable<Input<A>>;
  results: Writable<Results<A>>;
  generate: () => Promise<Input<A>>;
};

const ios: Partial<Record<ce_shell.Analysis, Io<ce_shell.Analysis>>> = {};

const initializeIo = <A extends ce_shell.Analysis>(
  analysis: A,
  defaultInput: Input<A>,
  defaultOutput: Output<A>,
): Io<A> => {
  const input = writable<Input<A>>(defaultInput);
  const results = writable<Results<A>>({
    outputState: 'None',
    output: defaultOutput,
    referenceOutput: defaultOutput,
    validation: null,
    latestJobId: null,
  });

  let activeJob: driver.job.JobId | null = null;
  let activeRequest: { abort: () => void } | null = null;

  const [debounceAnalysis] = debounce(
    (input: Input<A>) => api.analysis({ analysis, json: input }),
    200,
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

      results.update((s) => ({ ...s, outputState: 'Stale' }));

      const id = await (await debounceAnalysis(input)).data;
      activeJob = id;
      results.update((s) => ({ ...s, latestJobId: id }));
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
          results.update((s) => ({
            ...s,
            output: data.output.json as any,
            referenceOutput: data.reference_output.json as any,
            validation: data.validation as any,
            outputState: 'Current',
          }));
        } else if (result.kind == 'Failure') {
          results.update((s) => ({
            ...s,
            validation: { type: 'Failure', message: result.data.error },
            outputState: 'Current',
          }));
        }
      });
    },
  );

  const generate = () =>
    api.generate({ analysis }).data.then((result) => {
      input.set(result.json as any);
      return result.json as any;
    });

  if (browser) generate();

  return {
    input,
    results,
    generate,
  };
};

export const useIo = <A extends ce_shell.Analysis>(
  analysis: A,
  defaultInput: Input<A>,
  defaultOutput: Output<A>,
): Io<A> => {
  if (!ios[analysis]) {
    ios[analysis] = initializeIo(analysis, defaultInput, defaultOutput);
  }
  return ios[analysis] as Io<A>;
};

function debounce<A = unknown, R = void>(
  fn: (args: A) => R,
  ms: number,
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

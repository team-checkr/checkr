import { derived, writable, type Readable, type Writable } from 'svelte/store';
import { ce_shell, api, driver, type ce_core } from './api';
import { jobsStore, type Job, compilationStatusStore } from './events';
import { selectedJobId } from './jobs';
import { browser } from '$app/environment';

type Mapping = { [A in ce_shell.Analysis]: (ce_shell.Envs & { analysis: A })['io'] };

type OutputState = 'None' | 'Stale' | 'Current';

export type Input<A extends ce_shell.Analysis> = Mapping[A]['input'];
export type Output<A extends ce_shell.Analysis> = Mapping[A]['output'];

export type Results<A extends ce_shell.Analysis> = {
  outputState: OutputState;
  output: Output<A> | null;
  referenceOutput: Output<A> | null;
  validation: ce_core.ValidationResult | { type: 'Failure'; message: string } | null;
  job: Readable<Job> | null;
};

export type Io<A extends ce_shell.Analysis> = {
  input: Writable<Input<A>>;
  results: Readable<Results<A>>;
  generate: () => Promise<Input<A>>;
};

const ios: Partial<Record<ce_shell.Analysis, Io<ce_shell.Analysis>>> = {};

const initializeIo = <A extends ce_shell.Analysis>(analysis: A, defaultInput: Input<A>): Io<A> => {
  const input = writable<Input<A>>(defaultInput);

  const jobIdDerived = derived(
    [input, compilationStatusStore],
    ([$input, $compilationStatus], set) => {
      if (!browser) return;

      if ($compilationStatus?.state != 'Succeeded') return;

      let cancel = () => {};
      let stop = false;

      const run = async () => {
        await new Promise((resolve) => setTimeout(resolve, 200));
        if (stop) return;

        const analysisRequest = api.analysis({ analysis, json: $input });

        cancel = () => {
          analysisRequest.abort();
        };
        const jobId = await analysisRequest.data;
        cancel = () => {
          api.jobsCancel(jobId).data.catch(() => {});
        };

        set(jobId);
      };

      run();

      return () => {
        stop = true;
        cancel();
      };
    },
    null as number | null,
  );

  jobIdDerived.subscribe(($jobId) => {
    selectedJobId.set($jobId);
  });

  const results = derived(
    [jobIdDerived, jobsStore],
    ([$jobId, $jobs], set) => {
      if (typeof $jobId != 'number') return;

      let cancel = () => {};
      let stop = false;

      const run = async () => {
        set({
          outputState: 'None',
          output: null,
          referenceOutput: null,
          validation: null,
          job: null,
        } as Results<A>);

        let job = $jobs[$jobId];
        while (!stop && !job) {
          job = $jobs[$jobId];
          if (!job) await new Promise((resolve) => setTimeout(resolve, 200));
        }

        cancel = job.subscribe(($job) => {
          switch ($job.state) {
            case 'Succeeded':
              set({
                outputState: 'Current',
                // TODO: Add a toggle for showing the reference output in place of the output
                output: $job.analysis_data?.output?.json as any,
                referenceOutput: $job.analysis_data?.reference_output?.json as any,
                validation: $job.analysis_data?.validation as any,
                job,
              } as Results<A>);
              break;
            case 'Failed':
              set({
                outputState: 'Current',
                output: null,
                referenceOutput: null,
                validation: { type: 'Failure', message: $job.stdout },
                job,
              } as Results<A>);
          }
        });
      };

      run();

      return () => {
        stop = true;
        cancel();
      };
    },
    {
      outputState: 'None',
      output: null,
      referenceOutput: null,
      validation: null,
      job: null,
    } as Results<A>,
  );

  const generate = () =>
    api.generate({ analysis }).data.then((result) => {
      input.set(result.json as any);
      return result.json as any;
    });

  if (browser) generate();

  return {
    input,
    results: results,
    generate,
  };
};

export const useIo = <A extends ce_shell.Analysis>(analysis: A, defaultInput: Input<A>): Io<A> => {
  if (!ios[analysis]) {
    ios[analysis] = initializeIo(analysis, defaultInput);
  }
  return ios[analysis] as Io<A>;
};

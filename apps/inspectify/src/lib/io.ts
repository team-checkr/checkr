import { derived, writable, type Readable, type Writable } from 'svelte/store';
import { ce_shell, api, driver, type ce_core } from './api';
import { jobsStore, type Job, compilationStatusStore } from './events';
import { selectedJobId, showReference } from './jobs';
import { browser } from '$app/environment';

type Mapping = { [A in ce_shell.Analysis]: (ce_shell.Envs & { analysis: A })['io'] };

type OutputState = 'None' | 'Stale' | 'Current';

export type Input<A extends ce_shell.Analysis> = Mapping[A]['input'];
export type Output<A extends ce_shell.Analysis> = Mapping[A]['output'];
export type Meta<A extends ce_shell.Analysis> = Mapping[A]['meta'];

export type Results<A extends ce_shell.Analysis> = {
  input: Input<A>;
  outputState: OutputState;
  output: Output<A> | null;
  referenceOutput: Output<A> | null;
  validation: ce_core.ValidationResult | { type: 'Failure'; message: string } | null;
  job: Readable<Job> | null;
};

export type Io<A extends ce_shell.Analysis> = {
  input: Writable<Input<A>>;
  meta: Readable<Meta<A> | null>;
  results: Readable<Results<A>>;
  reference: Readable<Results<A>>;
  generate: () => Promise<Input<A>>;
};

const ios: Partial<Record<ce_shell.Analysis, Io<ce_shell.Analysis>>> = {};

const initializeIo = <A extends ce_shell.Analysis>(analysis: A, defaultInput: Input<A>): Io<A> => {
  const input = writable<Input<A>>(defaultInput);

  const jobIdAndInputDerived = derived(
    [input, compilationStatusStore, showReference],
    ([$input, $compilationStatus, $showReference], set) => {
      if (!browser) return;

      if ($showReference) return;

      if ($compilationStatus?.state != 'Succeeded') return;

      let cancel = () => {};
      let stop = false;

      const run = async () => {
        await new Promise((resolve) => setTimeout(resolve, 200));
        if (stop) return;

        const analysisRequest = api.analysis({
          analysis,
          json: $input,
          // TODO: we should avoid this somehow
          hash: { bytes: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0] },
        });

        cancel = () => {
          analysisRequest.abort();
        };
        const res = await analysisRequest.data;
        if (!res) return;
        cancel = () => {
          api.jobsCancel(res.id).data.catch(() => {});
        };

        set({ jobId: res.id, input: $input });
      };

      run();

      return () => {
        stop = true;
        cancel();
      };
    },
    null as { jobId: number; input: Input<A> } | null,
  );

  const jobIdDerived = derived(
    jobIdAndInputDerived,
    ($jobIdAndMeta) => $jobIdAndMeta?.jobId ?? null,
  );
  const cachedInput = derived(jobIdAndInputDerived, ($jobIdAndMeta) => $jobIdAndMeta?.input);

  jobIdDerived.subscribe(($jobId) => {
    selectedJobId.set($jobId);
  });

  const defaultResults: Results<A> = {
    input: defaultInput,
    outputState: 'None',
    output: null,
    referenceOutput: null,
    validation: null,
    job: null,
  };

  const results = derived(
    [jobIdDerived, cachedInput, jobsStore],
    ([$jobId, $cachedInput, $jobs], set) => {
      if (typeof $jobId != 'number' || !$cachedInput) return;

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

        let job = $jobs[$jobId] as Writable<Job> | undefined;
        while (!stop && !job) {
          job = $jobs[$jobId];
          if (!job) await new Promise((resolve) => setTimeout(resolve, 200));
        }

        if (!job) return;

        cancel = job.subscribe(($job) => {
          switch ($job.state) {
            case 'Succeeded':
              set({
                input: $cachedInput,
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
                input: $cachedInput,
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
    defaultResults,
  );

  const referenceAndMeta = derived(
    [input],
    ([$input], set) => {
      if (!browser) return;

      const analysisRequest = api.reference({
        analysis,
        json: $input,
        // TODO: we should avoid this somehow
        hash: { bytes: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0] },
      });

      analysisRequest.data.then(({ output, error, meta }) => {
        set({
          results: {
            input: $input,
            outputState: 'Current',
            output: output?.json as any,
            referenceOutput: output?.json as any,
            validation: { type: 'CorrectTerminated' },
            job: null,
          },
          meta: meta.json,
        });
      });

      return () => {
        analysisRequest.abort();
      };
    },
    { results: defaultResults, meta: null as Meta<A> | null },
  );
  const reference = derived(referenceAndMeta, ($referenceAndMeta) => $referenceAndMeta.results);
  const meta = derived(referenceAndMeta, ($referenceAndMeta) => $referenceAndMeta.meta);

  const generate = () =>
    api.generate({ analysis }).data.then((result) => {
      input.set(result.json as any);
      return result.json as any;
    });

  if (browser) generate();

  return {
    input,
    meta,
    results,
    reference,
    generate,
  };
};

export const useIo = <A extends ce_shell.Analysis>(analysis: A, defaultInput: Input<A>): Io<A> => {
  if (!ios[analysis]) {
    ios[analysis] = initializeIo(analysis, defaultInput);
  }
  return ios[analysis] as Io<A>;
};

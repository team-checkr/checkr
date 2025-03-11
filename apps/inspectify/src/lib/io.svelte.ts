import { ce_shell, api, type ce_core } from './api';
import { jobsStore, type Job, compilationStatus } from './events.svelte';
import { selectedJobId, showReference } from './jobs.svelte';
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
  job: Job | null;
};

const defaultResults = <A extends ce_shell.Analysis>(): Results<A> => ({
  input: null as any,
  outputState: 'None',
  output: null,
  referenceOutput: null,
  validation: null,
  job: null,
});

export class Io<A extends ce_shell.Analysis> {
  analysis: A;
  input: Input<A> = $state(null as any);
  meta: Meta<A> | null = $state(null);
  reference: Results<A> = $state(defaultResults());

  currentJob: { jobId: number; input: Input<A> } | null = $state(null);

  results: Results<A> = $derived.by<Results<A>>(() => {
    if (!this.currentJob || !(this.currentJob.jobId in jobsStore.jobs))
      return {
        input: this.input,
        outputState: 'None',
        job: null,
        output: null,
        referenceOutput: null,
        validation: null,
      } satisfies Results<A>;
    const job = jobsStore.jobs[this.currentJob.jobId];
    return {
      input: this.currentJob.input,
      outputState: job.analysis_data?.output?.json ? 'Current' : 'Stale',
      output: job.analysis_data?.output?.json as any,
      referenceOutput: job.analysis_data?.reference_output?.json as any,
      validation: job.analysis_data?.validation as any,
      job: job,
    } satisfies Results<A>;
  });

  constructor(analysis: A, defaultInput: Input<A>, seed?: number) {
    this.analysis = analysis;
    this.input = defaultInput;

    if (!browser) return;

    const params = new URLSearchParams(window.location.search);
    if (typeof seed != 'number') {
      const paramSeed = params.get('seed');
      if (typeof paramSeed == 'string') {
        seed = parseInt(paramSeed);
      }
    }

    // Kick off analysis
    $effect(() => {
      if (!browser) {
        return;
      }

      if (showReference.show) {
        return;
      }

      if (compilationStatus.status?.state != 'Succeeded') {
        return;
      }

      const inputSnapshot = $state.snapshot(this.input);

      let cancel = () => {};
      let stop = false;

      const run = async () => {
        await new Promise((resolve) => setTimeout(resolve, 200));
        if (stop) return;

        const analysisRequest = api.analysis({
          analysis,
          json: inputSnapshot,
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

        this.currentJob = { jobId: res.id, input: inputSnapshot };
      };

      run();

      return () => {
        stop = true;
        cancel();
      };
    });

    $effect(() => {
      if (this.currentJob) selectedJobId.jobId = this.currentJob.jobId;
    });

    $effect(() => {
      if (!browser) return;

      const analysisRequest = api.reference({
        analysis,
        json: this.input,
        // TODO: we should avoid this somehow
        hash: { bytes: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0] },
      });

      analysisRequest.data.then(({ output, error, meta }) => {
        this.meta = meta.json;
        this.reference = {
          input: this.input,
          outputState: 'Current',
          output: output?.json as any,
          referenceOutput: output?.json as any,
          validation: { type: 'CorrectTerminated' },
          job: null,
        };
      });

      return () => {
        analysisRequest.abort();
      };
    });

    this.generate(seed);
  }

  async generate(seed?: number): Promise<Input<A>> {
    const result = await api.generate({ analysis: this.analysis, seed: seed ?? null }).data;
    this.input = result.json as any;
    return result.json as any;
  }
}

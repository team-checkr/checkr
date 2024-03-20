import { browser } from '$app/environment';
import { readonly, writable, type Writable } from 'svelte/store';
import { api, driver, type inspectify } from './api';
import { produce } from 'immer';

export type Job = Omit<inspectify.endpoints.Job, 'kind'> & {
  kind: driver.job.JobKind | { kind: 'Waiting'; data: {} };
};

const jobsListWritableStore = writable<driver.job.JobId[]>([]);
export const jobsListStore = readonly(jobsListWritableStore);
export const jobsStore: Writable<Record<driver.job.JobId, Writable<Job>>> = writable({});

export const compilationStatusStore: Writable<inspectify.endpoints.CompilationStatus | null> =
  writable(null);

export const groupsConfigStore: Writable<inspectify.checko.config.GroupsConfig | null> =
  writable(null);
export const programsStore: Writable<inspectify.endpoints.Program[]> = writable([]);

export const groupProgramJobAssignedStore: Writable<
  Record<string, Record<string, driver.job.JobId>>
> = writable({});

type Connection = 'connected' | 'disconnected';

export const connectionStore: Writable<Connection> = writable('disconnected');

if (browser) {
  setTimeout(() => {
    api.events([]).listen((msg) => {
      if (msg.type == 'error') {
        connectionStore.set('disconnected');
        return;
      }
      connectionStore.set('connected');

      switch (msg.data.type) {
        case 'Reset': {
          jobsListWritableStore.set([]);
          jobsStore.set({});
          compilationStatusStore.set(null);
          groupsConfigStore.set(null);
          programsStore.set([]);
          groupProgramJobAssignedStore.set({});
          break;
        }
        case 'CompilationStatus': {
          compilationStatusStore.set(msg.data.value.status);
          break;
        }
        case 'JobChanged': {
          const { job } = msg.data.value;

          jobsStore.update(
            produce((jobsStore) => {
              if (!jobsStore[job.id]) {
                jobsStore[job.id] = writable(job);
              }
              jobsStore[job.id].set(job);
            }),
          );
          break;
        }
        case 'JobsChanged': {
          const { jobs } = msg.data.value;
          jobsListWritableStore.set(jobs);
          jobsStore.update(
            produce((jobsStore) => {
              for (const id of jobs) {
                if (!jobsStore[id]) {
                  jobsStore[id] = writable({
                    id,
                    state: 'Queued',
                    group_name: null,
                    kind: { kind: 'Waiting', data: {} },
                    stdout: '',
                    spans: [],
                    analysis_data: null,
                  });
                }
              }
            }),
          );
          break;
        }
        case 'GroupsConfig': {
          groupsConfigStore.set(msg.data.value.config);
          break;
        }
        case 'ProgramsConfig': {
          programsStore.set(msg.data.value.programs);
          break;
        }
        case 'GroupProgramJobAssigned': {
          const { group, program, job_id } = msg.data.value;
          groupProgramJobAssignedStore.update(
            produce((x) => {
              if (!x[group]) x[group] = {};
              x[group][program.hash_str] = job_id;
            }),
          );
          break;
        }
      }
    });
  }, 100);
}

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

export const compilationStatus: { status: inspectify.endpoints.CompilationStatus | null } = $state({
  status: null,
});

export const groupsConfigStore: { config: inspectify.checko.config.GroupsConfig | null } = $state({
  config: null,
});
export const programsStore: { programs: inspectify.endpoints.Program[] } = $state({ programs: [] });

export const groupProgramJobAssignedStore: {
  groups: Record<string, Record<string, driver.job.JobId>>;
} = $state({ groups: {} });

type Connection = 'connected' | 'disconnected';

export const connectionStore: { state: Connection } = $state({ state: 'disconnected' });

if (browser) {
  setTimeout(() => {
    api.events([]).listen((msg) => {
      if (msg.type == 'error') {
        connectionStore.state = 'disconnected';
        return;
      }
      connectionStore.state = 'connected';

      switch (msg.data.type) {
        case 'Reset': {
          jobsListWritableStore.set([]);
          jobsStore.set({});
          compilationStatus.status = null;
          groupsConfigStore.config = null;
          programsStore.programs = [];
          groupProgramJobAssignedStore.groups = {};
          break;
        }
        case 'CompilationStatus': {
          compilationStatus.status = msg.data.value.status;
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
          groupsConfigStore.config = msg.data.value.config;
          break;
        }
        case 'ProgramsConfig': {
          programsStore.programs = msg.data.value.programs;
          break;
        }
      }
    });
  }, 100);
}

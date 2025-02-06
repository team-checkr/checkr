import { browser } from '$app/environment';
import { api, driver, type inspectify } from './api';

export type Job = Omit<inspectify.endpoints.Job, 'kind'> & {
  kind: driver.job.JobKind | { kind: 'Waiting'; data: {} };
};

export const jobsListStore: { jobs: driver.job.JobId[] } = $state({ jobs: [] });
export const jobsStore: { jobs: Record<driver.job.JobId, Job> } = $state({ jobs: {} });

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
          jobsListStore.jobs = [];
          jobsStore.jobs = {};
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

          jobsStore.jobs[job.id] = job;
          break;
        }
        case 'JobsChanged': {
          const { jobs } = msg.data.value;
          jobsListStore.jobs = jobs;
          for (const id of jobs) {
            if (!jobsStore.jobs[id]) {
              jobsStore.jobs[id] = {
                id,
                state: 'Queued',
                group_name: null,
                kind: { kind: 'Waiting', data: {} },
                stdout: '',
                spans: [],
                analysis_data: null,
              };
            }
          }
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

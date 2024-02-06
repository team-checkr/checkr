import { browser } from '$app/environment';
import { readonly, writable, type Writable } from 'svelte/store';
import { api, driver, type inspectify_api } from './api';

const jobsListWritableStore = writable<driver.job.JobId[]>([]);
export const jobsListStore = readonly(jobsListWritableStore);
export const jobsStore: Record<
	driver.job.JobId,
	Writable<
		Omit<inspectify_api.endpoints.Job, 'kind'> & {
			kind: driver.job.JobKind | { kind: 'Waiting'; data: {} };
		}
	>
> = {};

export let compilationStatusStore: Writable<inspectify_api.endpoints.CompilationStatus | null> =
	writable(null);

type Connection = 'connected' | 'disconnected';

export const connectionStore: Writable<Connection> = writable('disconnected');

if (browser) {
	api.events([]).listen((msg) => {
		if (msg.type === 'error') {
			connectionStore.set('disconnected');
		}

		if (msg.type != 'message') return;
		connectionStore.set('connected');

		switch (msg.data.type) {
			case 'CompilationStatus': {
				compilationStatusStore.set(msg.data.value.status);
				break;
			}
			case 'JobChanged': {
				if (!jobsStore[msg.data.value.id]) {
					jobsStore[msg.data.value.id] = writable(msg.data.value.job);
				}
				jobsStore[msg.data.value.id].set(msg.data.value.job);
				break;
			}
			case 'JobsChanged': {
				jobsListWritableStore.set(msg.data.value.jobs);
				for (const id of msg.data.value.jobs) {
					if (!jobsStore[id]) {
						// TODO: perhaps add a kind that is more appropriate for unknown jobs
						jobsStore[id] = writable({
							id,
							state: 'Queued',
							group_name: null,
							kind: { kind: 'Waiting', data: {} },
							stdout: '',
							spans: [],
							analysis_data: null
						});
					}
				}
				break;
			}
		}
	});
}

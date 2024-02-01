import { writable, type Writable } from 'svelte/store';
import { api, type inspectify_api } from './api';
import { onMount } from 'svelte';

export let jobsStore: Writable<inspectify_api.endpoints.Job[]> = writable([]);

let isListeningToJobs = false;
export const startListeningOnJobs = () => {
	if (isListeningToJobs) return;
	isListeningToJobs = true;
	onMount(() => {
		api.jobs().listen((msg) => {
			if (msg.type == 'message') {
				jobsStore.set(msg.data);
			}
		});
	});
};

export let compilationStatusStore: Writable<inspectify_api.endpoints.CompilationStatus | null> =
	writable(null);

let isListeningToCompilation = false;
export const startListeningOnCompilation = () => {
	if (isListeningToCompilation) return;
	isListeningToCompilation = true;
	onMount(() => {
		api.compilationStatus().listen((msg) => {
			if (msg.type == 'message') {
				compilationStatusStore.set(msg.data);
			}
		});
	});
};

import { writable, type Writable } from 'svelte/store';
import { api, type inspectify_api } from './api';
import { onMount } from 'svelte';

export let jobsStore: Writable<inspectify_api.Job[]> = writable([]);

let isListening = false;
export const startListeningOnJobs = () => {
	if (isListening) return;
	isListening = true;
	onMount(() => {
		api.jobs().listen((msg) => {
			if (msg.type == 'message') {
				jobsStore.set(msg.data);
			}
		});
	});
};

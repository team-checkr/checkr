// import { readable, writable, type Readable, type Writable, derived } from 'svelte/store';
// import { api, driver, type inspectify_api } from './api';
// import { onMount } from 'svelte';

// export const jobList = readable<driver.job.JobId[]>([], (set) => {
// 	const stream = api.jobsList([]);

// 	const id = Date.now();
// 	console.log('listening to job list', id);
// 	stream.listen((msg) => {
// 		if (msg.type == 'message') set(msg.data);
// 	});
// 	return () => {
// 		console.log('cancelling job list', id);
// 		stream.cancel();
// 	};
// });

// export const jobsStore: Record<driver.job.JobId, Readable<inspectify_api.endpoints.Job>> = {};

// export const jobStore = (id: driver.job.JobId) => {
// 	if (!jobsStore[id]) {
// 		jobsStore[id] = readable<inspectify_api.endpoints.Job>(
// 			{
// 				id,
// 				state: 'Queued',
// 				kind: { kind: 'Compilation', data: {} },
// 				stdout: '',
// 				spans: []
// 			},
// 			(set) => {
// 				const stream = api.jobsIdEvents([id]);
// 				stream.listen((msg) => {
// 					if (msg.type == 'message') set(msg.data);
// 				});
// 				return () => stream.cancel();
// 			}
// 		);
// 	}
// 	return jobsStore[id];
// };

// // export const jobsStore: Writable<inspectify_api.endpoints.Job[]> = writable([]);

// // let isListeningToJobs = false;
// // export const startListeningOnJobs = () => {
// // 	if (isListeningToJobs) return;
// // 	isListeningToJobs = true;
// // 	onMount(() => {
// // 		const stream = api.jobs();
// // 		stream.listen((msg) => {
// // 			console.log(msg, Date.now());
// // 			if (msg.type == 'message') {
// // 				jobsStore.set(msg.data);
// // 			}
// // 		});
// // 		return () => stream.cancel();
// // 	});
// // };

// export let compilationStatusStore: Writable<inspectify_api.endpoints.CompilationStatus | null> =
// 	writable(null);

// let isListeningToCompilation = false;
// export const startListeningOnCompilation = () => {
// 	if (isListeningToCompilation) return;
// 	isListeningToCompilation = true;
// 	onMount(() => {
// 		const stream = api.compilationStatus([]);
// 		stream.listen((msg) => {
// 			if (msg.type == 'message') {
// 				compilationStatusStore.set(msg.data);
// 			}
// 		});
// 		return () => stream.cancel();
// 	});
// };

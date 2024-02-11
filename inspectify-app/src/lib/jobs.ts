import { writable } from 'svelte/store';
import type { driver } from '$lib/api';

export const showStatus = writable(false);

export const selectedJobId = writable<driver.job.JobId | null>(null);

export const tabs = [
	'Output',
	'Input JSON',
	'Output JSON',
	'Reference Output',
	'Validation'
] as const;
export type Tab = (typeof tabs)[number];
export const currentTab = writable<Tab>('Output');

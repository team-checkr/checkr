import type { driver } from '$lib/api';

export const showStatus: { show: boolean } = $state({ show: false });

export const showReference: { show: boolean } = $state({ show: false });

export const selectedJobId: { jobId: driver.job.JobId | null } = $state({ jobId: null });

export const tabs = [
  'Input JSON',
  'Output',
  'Output JSON',
  'Reference Output',
  'Validation',
] as const;
export type Tab = (typeof tabs)[number];
export const currentTab: { current: Tab } = $state({ current: 'Output' });

import { browser } from '$app/environment';
import { readonly, writable } from 'svelte/store';
import { api, type inspectify } from './api';

const publicDataAnalysis = writable<inspectify.endpoints.PublicAnalysis[]>([]);
export const publicDataAnalysisStore = readonly(publicDataAnalysis);

const publicDataGroups = writable<inspectify.endpoints.PublicGroup[]>([]);
export const publicDataGroupsStore = readonly(publicDataGroups);

if (browser) {
  setTimeout(() => {
    api.checkoPublic([]).listen((msg) => {
      if (msg.type == 'error') {
        return;
      }

      switch (msg.data.type) {
        case 'Reset': {
          publicDataAnalysis.set([]);
          publicDataGroups.set([]);
          break;
        }
        case 'StateChanged': {
          publicDataAnalysis.set(msg.data.value.analysis);
          publicDataGroups.set(msg.data.value.groups);
          break;
        }
      }
    });
  }, 100);
}

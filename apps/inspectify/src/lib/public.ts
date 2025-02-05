import { browser } from '$app/environment';
import { readonly, writable } from 'svelte/store';
import { api, type inspectify } from './api';

const publicDataAnalysis = writable<inspectify.checko.scoreboard.PublicAnalysis[]>([]);
export const publicDataAnalysisStore = readonly(publicDataAnalysis);

const publicDataGroups = writable<inspectify.checko.scoreboard.PublicGroup[]>([]);
export const publicDataGroupsStore = readonly(publicDataGroups);

const lastFinished = writable<Date | null>(null);
export const lastFinishedStore = readonly(lastFinished);

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
          lastFinished.set(null);
          break;
        }
        case 'StateChanged': {
          publicDataAnalysis.set(msg.data.value.analysis);
          publicDataGroups.set(msg.data.value.groups);
          if (msg.data.value.last_finished) {
            lastFinished.set(new Date(msg.data.value.last_finished));
          }
          break;
        }
      }
    });
  }, 100);
}

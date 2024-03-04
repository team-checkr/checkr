import { browser } from '$app/environment';
import { readonly, writable } from 'svelte/store';
import { api, type inspectify } from './api';

const publicDataStore = writable<inspectify.endpoints.PublicState>({
  analysis: [],
  groups: [],
});
export const publicData = readonly(publicDataStore);

if (browser) {
  setTimeout(() => {
    api.checkoPublic([]).listen((msg) => {
      if (msg.type == 'error') {
        return;
      }

      switch (msg.data.type) {
        case 'Reset': {
          publicDataStore.set({
            analysis: [],
            groups: [],
          });
          break;
        }
        case 'StateChanged': {
          publicDataStore.set(msg.data.value);
          break;
        }
      }
    });
  }, 100);
}

import { browser } from '$app/environment';
import { api, type inspectify } from './api';

export const publicData: {
  analysis: inspectify.checko.scoreboard.PublicAnalysis[];
  groups: inspectify.checko.scoreboard.PublicGroup[];
  lastFinished: Date | null;
} = $state({
  analysis: [],
  groups: [],
  lastFinished: null,
});

if (browser) {
  setTimeout(() => {
    api.checkoPublic([]).listen((msg) => {
      if (msg.type == 'error') {
        return;
      }

      switch (msg.data.type) {
        case 'Reset': {
          publicData.analysis = [];
          publicData.groups = [];
          publicData.lastFinished = null;
          break;
        }
        case 'StateChanged': {
          publicData.analysis = msg.data.value.analysis;
          publicData.groups = msg.data.value.groups;
          if (msg.data.value.last_finished) {
            publicData.lastFinished = new Date(msg.data.value.last_finished);
          }
          break;
        }
      }
    });
  }, 100);
}

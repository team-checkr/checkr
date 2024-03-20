<script lang="ts">
  import { browser } from '$app/environment';
  import { goto } from '$app/navigation';
  import { ce_shell } from '$lib/api';
  import { PUBLIC_CHECKO } from '$env/static/public';

  if (browser) {
    if (PUBLIC_CHECKO) {
      goto(`/scoreboard`, { replaceState: true });
    } else {
      goto(`/env/${ce_shell.ANALYSIS[0]}`, { replaceState: true });
    }
  }
</script>

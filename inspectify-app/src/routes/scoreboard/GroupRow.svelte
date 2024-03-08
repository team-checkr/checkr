<script lang="ts">
  import type { driver, inspectify } from '$lib/api';
  import { onMount } from 'svelte';

  export let numberOfPrograms: number;
  export let group: inspectify.checko.public.PublicGroup;

  let canvas: HTMLCanvasElement;

  $: {
    if (canvas) {
      const ctx = canvas.getContext('2d')!;

      const draw = async () => {
        const { default: config } = await import('tailwind.config.ts');

        const colors: Record<driver.job.JobState, string> = {
          Queued: '',
          Running: config.theme.colors['slate'][400],
          Succeeded: config.theme.colors['green'][500],
          Canceled: config.theme.colors['slate'][400],
          Failed: config.theme.colors['red'][500],
          Warning: config.theme.colors['yellow'][400],
          Timeout: config.theme.colors['blue'][400],
          OutputLimitExceeded: config.theme.colors['orange'][400],
        };

        ctx.clearRect(0, 0, canvas.width, canvas.height);

        const cellWidth = Math.round(canvas.width / numberOfPrograms);
        const cellHeight = canvas.height;

        const borderColor = config.theme.colors['slate'][800] + '10';

        let index = 0;
        for (const analysis of group.analysis_results) {
          for (const res of analysis.results) {
            const idx = index++;
            ctx.fillStyle = colors[res.state] || config.theme.colors['slate'][700];
            ctx.fillRect(idx * cellWidth, 0, cellWidth, cellHeight);
            // Draw right and bottom borders
            ctx.fillStyle = borderColor;
            ctx.fillRect(idx * cellWidth + cellWidth - 1, 0, 1, cellHeight);
          }
        }
        ctx.fillStyle = borderColor;
        ctx.fillRect(0, cellHeight - 1, canvas.width, 1);
      };

      draw();
    }
  }

  onMount(() => {
    const observer = new ResizeObserver(() => {
      canvas.width = canvas.clientWidth;
      canvas.height = canvas.clientHeight;
    });
    observer.observe(canvas);
    return () => observer.disconnect();
  });
</script>

<div class="flex items-center justify-center border bg-slate-800 px-1 font-mono text-xs font-bold">
  {group.name}
</div>
<canvas class="h-6 w-full" bind:this={canvas} />

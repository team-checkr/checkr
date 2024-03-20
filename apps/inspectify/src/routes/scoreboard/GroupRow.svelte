<script lang="ts">
  import type { driver, inspectify } from '$lib/api';
  import { onMount } from 'svelte';

  export let numberOfPrograms: number;
  export let group: inspectify.checko.scoreboard.PublicGroup;
  export let analysisStore: inspectify.checko.scoreboard.PublicAnalysis[];

  $: accLength = analysisStore.reduce(
    (acc, analysis) => {
      return [...acc, acc[acc.length - 1] + analysis.programs.length];
    },
    [2],
  );

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

        const t = Date.now() / 5000;
        let redrawing = false;
        let index = 0;
        for (const analysis of group.analysis_results) {
          for (const res of analysis.results) {
            const isWorking =
              analysis.status != 'Finished' && analysis.status != 'CompilationError';
            if (isWorking) {
              redrawing = true;
            }
            const idx = index++;
            ctx.fillStyle = colors[res.state] || config.theme.colors['slate'][700];
            if (isWorking) {
              const suffix = `${(Math.pow(Math.sin(index / 50 + t) / Math.PI, 2) * 10000).toString(
                16,
              )}`.slice(0, 2);
              ctx.fillStyle += suffix;
            }
            ctx.fillRect(idx * cellWidth, 0, cellWidth, cellHeight);
            // Draw right and top borders
            ctx.fillStyle = borderColor;
            ctx.fillRect(idx * cellWidth + cellWidth - 1, 0, 1, cellHeight);
          }
        }
        ctx.fillStyle = borderColor;
        ctx.fillRect(0, cellHeight - 1, canvas.width, 1);

        if (redrawing) requestAnimationFrame(draw);
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

<div
  class="row-start-1 flex items-center justify-center border bg-slate-800 px-1 font-mono text-xs font-bold"
>
  {group.name}
</div>
<canvas
  class="row-start-1 h-6 w-full"
  style="grid-column: 2 / span {numberOfPrograms};"
  bind:this={canvas}
/>
{#each group.analysis_results as analysis, index}
  <div
    class="row-start-1 mt-px flex items-center font-mono text-xs {analysis.status == 'Finished'
      ? 'opacity-10'
      : 'opacity-50'} transition group-hover:opacity-100"
    style="grid-column: {accLength[index]};"
  >
    <span class="absolute">
      {analysis.status}
      {analysis.last_hash ? `- ${analysis.last_hash.slice(0, 7)}` : ''}
    </span>
  </div>
{/each}

<script lang="ts">
  import type { driver, inspectify } from '$lib/api';
  import { onMount } from 'svelte';

  interface Props {
    numberOfPrograms: number;
    group: inspectify.checko.scoreboard.PublicGroup;
    analysisStore: inspectify.checko.scoreboard.PublicAnalysis[];
  }

  let { numberOfPrograms, group, analysisStore }: Props = $props();

  let accLength = $derived(
    analysisStore.reduce(
      (acc, analysis) => [...acc, acc[acc.length - 1] + analysis.programs.length],
      [2],
    ),
  );

  const getColor = (name: string) =>
    getComputedStyle(document.documentElement).getPropertyValue(name);

  const colors: Record<driver.job.JobState, string> = {
    Queued: '',
    Running: getColor('--color-slate-400'),
    Succeeded: getColor('--color-green-500'),
    Canceled: getColor('--color-slate-400'),
    Failed: getColor('--color-red-500'),
    Warning: getColor('--color-yellow-400'),
    Timeout: getColor('--color-blue-400'),
    OutputLimitExceeded: getColor('--color-orange-400'),
  };

  const neutralColor = getColor('--color-slate-700');
  const borderColor = getColor('--color-slate-800');

  let canvas: HTMLCanvasElement | undefined = $state();

  const draw = $derived.by(() => {
    for (const analysis of group.analysis_results) {
      $inspect(analysis.status);
    }

    return async () => {
      if (!canvas) return;

      const ctx = canvas.getContext('2d')!;

      ctx.clearRect(0, 0, canvas.width, canvas.height);

      const cellWidth = Math.floor(canvas.width / numberOfPrograms);
      const cellHeight = canvas.height;

      const t = Date.now() / 5000;
      let redrawing = false;
      let index = 0;
      let currentWidthDrawn = 0;
      for (const analysis of group.analysis_results) {
        for (const res of analysis.results) {
          const isWorking = analysis.status != 'Finished' && analysis.status != 'CompilationError';
          if (isWorking) {
            redrawing = true;
          }
          const idx = index++;
          ctx.fillStyle = colors[res.state] || neutralColor;
          if (isWorking) {
            const suffix = `${(Math.pow(Math.sin(index / 50 + t) / Math.PI, 2) * 10000).toString(
              16,
            )}`.slice(0, 2);
            ctx.fillStyle += suffix;
          }
          const expectedWithDrawn = (canvas.width / numberOfPrograms) * idx;
          const thisCellWidth = currentWidthDrawn < expectedWithDrawn ? cellWidth + 1 : cellWidth;
          ctx.fillRect(currentWidthDrawn, 0, thisCellWidth, cellHeight);
          // Draw right and top borders
          ctx.globalAlpha = 0.1;
          ctx.fillStyle = borderColor;
          ctx.fillRect(currentWidthDrawn + thisCellWidth - 1, 0, 1, cellHeight);
          ctx.globalAlpha = 1.0;
          currentWidthDrawn += thisCellWidth;
        }
      }
      ctx.globalAlpha = 0.1;
      ctx.fillStyle = borderColor;
      ctx.fillRect(0, cellHeight - 1, canvas.width, 1);
      ctx.globalAlpha = 1.0;

      if (redrawing) requestAnimationFrame(draw);
    };
  });

  $effect(() => {
    if (!canvas) return;

    setTimeout(draw, 100);
  });

  onMount(() => {
    if (!canvas) return;
    const observer = new ResizeObserver(() => {
      if (!canvas) return;
      canvas.width = canvas.clientWidth;
      canvas.height = canvas.clientHeight;
      requestAnimationFrame(draw);
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
></canvas>
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

<div class="flex min-h-screen grid-cols-2 bg-slate-800 p-10">
  {#each [{ title: 'Chip', sub: 'Program verification' }, { title: 'Moka', sub: 'Model checking' }] as page, i}
    {#if 0 < i}
      <div class="mx-10 flex w-px">
        <div class="w-px bg-slate-600"></div>
      </div>
    {/if}
    <a
      class="group flex flex-1 flex-col items-center justify-center gap-5 rounded-xl text-white transition hover:bg-slate-200/10"
      href={page.title.toLowerCase()}
    >
      <span class="text-5xl font-thin italic transition group-hover:scale-105">
        {page.title}
      </span>
      <span class="font-thin tracking-wider text-slate-100">{page.sub} </span>
    </a>
  {/each}
</div>

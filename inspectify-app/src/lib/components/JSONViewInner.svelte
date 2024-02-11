<script lang="ts">
	export let json: any;
</script>

{#if typeof json == 'string'}
	<pre class="font-mono text-slate-200">{json}</pre>
{:else if typeof json == 'number'}
	<pre class="font-mono text-purple-600">{json}</pre>
{:else if typeof json == 'boolean'}
	<pre class="font-mono text-blue-600">{json}</pre>
{:else if Array.isArray(json)}
	<div>
		{#each json as value (value)}
			<div class="border px-2 py-1">
				<svelte:self json={value} />
			</div>
		{/each}
	</div>
{:else if !json}
	<pre class="text-gray-400">null</pre>
{:else if typeof json == 'object'}
	<div class="flex flex-col space-y-2">
		{#each Object.entries(json) as [key, value] (key)}
			{#if typeof value == 'string' && !value.includes('\n')}
				<div class="flex items-baseline">
					<div class="px-2 py-0.5 text-sm font-bold">{key}:</div>
					<div class="ml-2 px-2 py-1">
						<svelte:self json={value} />
					</div>
				</div>
			{:else}
				<div>
					<div class="px-2 py-0.5 text-sm font-bold">{key}:</div>
					<div class="ml-2 border px-2 py-1">
						<svelte:self json={value} />
					</div>
				</div>
			{/if}
		{/each}
	</div>
{:else}
	<pre class="text-red-600">{JSON.stringify(json, null, 2)}</pre>
{/if}

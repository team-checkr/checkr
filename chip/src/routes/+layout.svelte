<script>
	import '../app.pcss';

	import Icon from '~icons/heroicons/check-badge';
</script>

<div class="grid h-screen grid-rows-[auto_1fr]">
	<nav class="flex items-center bg-slate-900 px-2 text-slate-200">
		<a href="/" class="flex items-center space-x-2 p-2 pr-0 text-2xl font-thin italic">
			<div class="relative">
				<Icon class="absolute inset-0 left-0.5 top-0.5 w-6 animate-pulse text-teal-500/50" />
				<Icon class="relative w-6" />
			</div>
			<span>Chip</span>
		</a>
	</nav>

	<slot />
</div>

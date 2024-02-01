<script lang="ts">
	import type { inspectify_api } from '$lib/api';
	export let spans: inspectify_api.Span[];
</script>

{#each spans as span}<span class="fg-{span.fg} bg-{span.bg}">{span.text}</span>{/each}

<style>
	/* Foregrounds */
	.fg-Black {
		@apply text-black;
	}
	.fg-Red {
		@apply text-red-500;
	}
	.fg-Green {
		@apply text-green-500;
	}
	.fg-Yellow {
		@apply text-yellow-500;
	}
	.fg-Blue {
		@apply text-blue-500;
	}
	.fg-Magenta {
		@apply text-purple-500;
	}
	.fg-Cyan {
		@apply text-cyan-500;
	}
	.fg-White {
		@apply text-white;
	}
	.fg-Default {
	}
	.fg-BrightBlack {
		@apply text-black;
	}
	.fg-BrightRed {
		@apply text-red-300;
	}
	.fg-BrightGreen {
		@apply text-green-300;
	}
	.fg-BrightYellow {
		@apply text-yellow-300;
	}
	.fg-BrightBlue {
		@apply text-blue-300;
	}
	.fg-BrightMagenta {
		@apply text-purple-300;
	}
	.fg-BrightCyan {
		@apply text-cyan-300;
	}
	.fg-BrightWhite {
		@apply text-white;
	}

	/* Backgrounds */
	.bg-Black {
		@apply bg-black;
	}
	.bg-Red {
		@apply bg-red-500;
	}
	.bg-Green {
		@apply bg-green-500;
	}
	.bg-Yellow {
		@apply bg-yellow-500;
	}
	.bg-Blue {
		@apply bg-blue-500;
	}
	.bg-Magenta {
		@apply bg-purple-500;
	}
	.bg-Cyan {
		@apply bg-cyan-500;
	}
	.bg-White {
		@apply bg-white;
	}
</style>

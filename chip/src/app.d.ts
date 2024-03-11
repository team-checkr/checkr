import 'unplugin-icons/types/svelte';

// See https://kit.svelte.dev/docs/types#app
// for information about these interfaces
declare global {
	namespace App {
		// interface Error {}
		// interface Locals {}
		// interface PageData {}
		// interface PageState {}
		// interface Platform {}
	}
}

declare module 'z3-solver/build/z3-built' {
	export default function (opts: {
		locateFile: (f: string) => f;
		mainScriptUrlOrBlob: string;
	}): any;
}

export {};

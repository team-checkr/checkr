<script lang="ts" generics="T">
  import { run } from 'svelte/legacy';

  let input: string = $state('');

  interface Props {
    value: T;
    type: T extends number ? 'int' : T extends Array<infer S> ? 'array' : 'who knows';
    stringify?: any;
    parse?: any;
  }

  let {
    value = $bindable(),
    type,
    stringify = (x: T) => JSON.stringify(x),
    parse = (x: string) => {
      try {
        const val = JSON.parse(x);
        switch (type) {
          case 'int':
            if (typeof val === 'number') return Math.floor(val) as T;
            break;
          case 'array':
            if (Array.isArray(val)) return val as T;
            break;
        }
      } catch (err) {}
    },
  }: Props = $props();

  const g = (x: T) => {
    const res = stringify(x);
    if (typeof res != 'undefined') input = res;
  };
  const gInv = (x: string) => {
    const res = parse(x);
    if (typeof res != 'undefined') value = res;
  };
  run(() => {
    g(value);
  });
  run(() => {
    gInv(input);
  });
</script>

<input
  type="text"
  class="w-full border-x-0 border-b border-t-0 bg-transparent outline-none"
  bind:value={input}
/>

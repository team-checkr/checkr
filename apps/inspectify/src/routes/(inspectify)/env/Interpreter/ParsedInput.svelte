<script lang="ts" generics="T">
  import type { ChangeEventHandler } from 'svelte/elements';

  interface Props {
    value: T;
    type: T extends number ? 'int' : T extends Array<infer S> ? 'array' : 'who knows';
    stringify?: (x: T) => string;
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

  let setValue = $state(value);

  let input: string = $state(stringify(value));

  const onChange: ChangeEventHandler<HTMLInputElement> = $derived(() => {
    const res = parse(input);
    if (typeof res == 'undefined') return;
    if (stringify(res) != stringify(value)) {
      value = res;
      setValue = res;
    }
  });
  $effect(() => {
    if (stringify(value) != stringify(setValue)) {
      input = stringify(value);
      setValue = value;
    }
  });
</script>

<input
  type="text"
  class="outline-hidden w-full border-x-0 border-b border-t-0 bg-transparent"
  bind:value={input}
  oninput={onChange}
/>

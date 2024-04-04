<script lang="ts" generics="T">
  export let value: T;
  export let type: T extends number ? 'int' : T extends Array<infer S> ? 'array' : 'who knows';
  let input: string = '';

  export let stringify = (x: T) => JSON.stringify(x);
  export let parse = (x: string) => {
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
  };

  const g = (x: T) => {
    const res = stringify(x);
    if (typeof res != 'undefined') input = res;
  };
  const gInv = (x: string) => {
    const res = parse(x);
    if (typeof res != 'undefined') value = res;
  };
  $: g(value);
  $: gInv(input);
</script>

<input
  type="text"
  class="w-full border-x-0 border-b border-t-0 bg-transparent outline-none"
  bind:value={input}
/>

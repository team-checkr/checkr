<script lang="ts" generics="T">
  export let value: T;
  export let type: T extends number ? 'int' : 'array';
  let input: string = '';

  const f = (x: T) => (input = JSON.stringify(x));
  const fInv = (x: string) => {
    try {
      const val = JSON.parse(x);
      switch (type) {
        case 'int':
          if (typeof val === 'number') value = Math.floor(val) as T;
          break;
        case 'array':
          if (Array.isArray(val)) value = val as T;
          break;
      }
    } catch (err) {}
  };

  $: f(value);
  $: fInv(input);
</script>

<input
  type="text"
  class="border-x-0 border-b border-t-0 bg-transparent outline-none"
  bind:value={input}
/>

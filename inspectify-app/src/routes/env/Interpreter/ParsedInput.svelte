<script lang="ts" generics="T">
  export let value: T;
  let input: string = '';

  const f = (x: T) => (input = JSON.stringify(x));
  const fInv = (x: string) => {
    try {
      value = JSON.parse(x);
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

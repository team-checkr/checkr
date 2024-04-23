<script lang="ts" generics="T">
  export let name: string;
  export let id: string;
  export let value: T;
  export let array: T[];

  let input: HTMLInputElement;
</script>

<input
  bind:this={input}
  type="checkbox"
  {name}
  {id}
  checked={array.includes(value)}
  on:change={() => (array = input.checked ? [...array, value] : array.filter((x) => x != value))}
/>

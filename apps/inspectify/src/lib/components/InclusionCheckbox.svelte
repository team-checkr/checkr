<script lang="ts" generics="T">
  interface Props {
    name: string;
    id: string;
    value: T;
    array: T[];
  }

  let { name, id, value, array = $bindable() }: Props = $props();

  let input: HTMLInputElement | undefined = $state();
</script>

<input
  bind:this={input}
  type="checkbox"
  {name}
  {id}
  checked={array.includes(value)}
  onchange={() => (array = input?.checked ? [...array, value] : array.filter((x) => x != value))}
/>

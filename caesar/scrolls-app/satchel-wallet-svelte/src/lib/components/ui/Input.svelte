<script>
  import { cn } from "../../utils/cn.js";
  
  let className = "";
  export { className as class };
  export let value = "";
  export let type = "text";
  export let placeholder = "";
  export let disabled = false;
  export let required = false;
  
  $: classes = cn(
    "flex h-10 w-full rounded-md border px-3 py-2 text-sm transition-colors",
    "border-gray-600 bg-gray-900 text-white placeholder:text-gray-400",
    "focus:border-yellow-400 focus:outline-none focus:ring-2 focus:ring-yellow-400/20",
    "disabled:cursor-not-allowed disabled:opacity-50",
    className
  );
</script>

<input
  class={classes}
  bind:value
  {type}
  {placeholder}
  {disabled}
  {required}
  {...$$restProps}
  on:change
  on:input
  on:focus
  on:blur
/>
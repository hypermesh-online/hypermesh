<script>
  import { cn } from "../../utils/cn.js";

  export let variant = "default";
  export let size = "default";
  export let disabled = false;
  export let type = "button";
  
  const variants = {
    default: "bg-gradient-to-r from-yellow-400 to-yellow-600 hover:from-yellow-500 hover:to-yellow-700 text-black font-semibold",
    secondary: "bg-gray-800 hover:bg-gray-700 text-white border border-gray-600",
    outline: "border-2 border-yellow-400 text-yellow-400 hover:bg-yellow-400 hover:text-black bg-transparent",
    ghost: "hover:bg-gray-800 text-gray-300 hover:text-white bg-transparent",
    destructive: "bg-red-600 hover:bg-red-700 text-white"
  };
  
  const sizes = {
    default: "h-10 px-4 py-2",
    sm: "h-9 rounded-md px-3 text-sm",
    lg: "h-11 rounded-md px-8 text-base",
    icon: "h-10 w-10"
  };
  
  $: classes = cn(
    "inline-flex items-center justify-center rounded-md text-sm font-medium transition-colors",
    "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-yellow-400",
    "disabled:pointer-events-none disabled:opacity-50",
    variants[variant],
    sizes[size]
  );
</script>

<button 
  class={classes}
  {type}
  {disabled}
  on:click
  {...$$restProps}
>
  <slot />
</button>
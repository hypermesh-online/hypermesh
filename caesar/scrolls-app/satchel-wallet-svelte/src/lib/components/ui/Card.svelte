<script>
  import { cn } from "../../utils/cn.js";
  
  let className = "";
  export { className as class };
</script>

<div class={cn("wallet-card p-6", className)} {...$$restProps}>
  <slot />
</div>
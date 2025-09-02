<!-- 
  Reusable Help Link Component
  Small underlined link that triggers help sidebar with subtle hover effect
  Consistent styling for all help links throughout the app
-->

<script lang="ts">
  let { 
    text,
    onclick 
  } = $props();
</script>

<button 
  {onclick}
  class="text-primary text-xs underline decoration-dotted underline-offset-4 dark:text-blue-400 
         hover:bg-primary/5 dark:hover:bg-blue-400/10 
         px-2 py-1 rounded-md transition-colors duration-200"
>
  {text}
</button>

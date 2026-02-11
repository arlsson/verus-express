<!-- 
  Component: BlurredSeedGrid
  Purpose: Display seed words in a clear grid layout
  Last Updated: Removed hover-to-reveal blur for easier testing
  Security: Words always visible (blur protection disabled for development)
-->

<script lang="ts">
  // Props
  let { 
    seedWords = [],
    startIndex = 0,
    showNumbers = true 
  } = $props();
</script>

<div class="relative">
  <!-- Grid Container -->
  <div class="grid grid-cols-2 gap-2 rounded-xl bg-muted/30 p-3">
    {#each seedWords as word, index}
      {@const wordNumber = startIndex + index + 1}
      
      <div class="relative rounded-md border border-border bg-background p-2.5">
        <!-- Word Number Badge -->
        {#if showNumbers}
          <div
            class="absolute -left-2 -top-2 flex h-5 w-5 items-center justify-center rounded-full border border-border bg-muted text-xs font-bold text-foreground"
          >
            {wordNumber}
          </div>
        {/if}
        
        <!-- Word Display -->
        <div class="flex min-h-[1.5rem] items-center justify-center">
          <span class="font-mono text-base font-semibold text-card-foreground">
            {word}
          </span>
        </div>
      </div>
    {/each}
  </div>
</div>

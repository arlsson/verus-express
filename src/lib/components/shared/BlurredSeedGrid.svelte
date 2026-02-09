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
  <div class="grid grid-cols-2 gap-3 p-6 bg-muted/30 rounded-lg">
    {#each seedWords as word, index}
      {@const wordNumber = startIndex + index + 1}
      
      <div class="relative rounded-lg bg-background border border-border p-4">
        <!-- Word Number Badge -->
        {#if showNumbers}
          <div class="absolute -top-2 -left-2 w-6 h-6 bg-primary text-primary-foreground 
                      rounded-full flex items-center justify-center text-xs font-bold">
            {wordNumber}
          </div>
        {/if}
        
        <!-- Word Display -->
        <div class="flex items-center justify-center min-h-[2rem]">
          <span class="font-mono text-lg font-semibold text-card-foreground">
            {word}
          </span>
        </div>
      </div>
    {/each}
  </div>
</div>

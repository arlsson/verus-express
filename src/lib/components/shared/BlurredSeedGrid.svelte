<!-- 
  Component: BlurredSeedGrid
  Purpose: Display seed words with group hover-to-reveal blur protection
  Last Updated: Updated to show/hide entire group instead of individual words
  Security: Group-level blur protection against shoulder surfing and screenshots
-->

<script lang="ts">
  // Props
  let { 
    seedWords = [],
    startIndex = 0,
    showNumbers = true 
  } = $props();
  
  // Local state for group hover
  let isGroupHovered = $state(false);
  
  function handleGroupMouseEnter() {
    isGroupHovered = true;
  }
  
  function handleGroupMouseLeave() {
    isGroupHovered = false;
  }
</script>

<div 
  class="relative cursor-pointer group"
  onmouseenter={handleGroupMouseEnter}
  onmouseleave={handleGroupMouseLeave}
  role="button"
  tabindex="0"
>
  <!-- Grid Container -->
  <div class="grid grid-cols-2 gap-3 p-6 bg-muted/30 rounded-lg transition-all duration-200
              {isGroupHovered ? 'bg-muted/40' : 'bg-muted/30'}">
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
          <span 
            class="font-mono text-lg font-semibold text-card-foreground transition-all duration-200
                   {isGroupHovered ? 'filter-none' : 'filter blur-sm'}"
          >
            {word}
          </span>
        </div>
      </div>
    {/each}
  </div>
  
  <!-- Group Blur Overlay -->
  {#if !isGroupHovered}
    <div class="absolute inset-0 flex items-center justify-center 
                bg-background/60 rounded-lg backdrop-blur-[2px]">
      <div class="bg-background/90 border border-border rounded-lg px-4 py-2">
        <span class="text-sm text-muted-foreground font-medium">
          🔒 Hover to reveal words
        </span>
      </div>
    </div>
  {/if}
</div>

<style>
  .filter {
    filter: blur(4px);
  }
  .filter-none {
    filter: none;
  }
</style>

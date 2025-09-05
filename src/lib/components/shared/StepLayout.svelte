<!-- 
  Component: StepLayout
  Purpose: Reusable 2-column layout with sticky bottom action for wallet creation steps
  Last Updated: Created for DRY layout architecture
  Security: No sensitive data - pure layout component
-->

<!-- 2-Column Layout + Sticky Bottom -->
<div class="h-full flex">
  <!-- Left Column: Step Info (fixed 250px width, sticky) -->
  <div class="w-[250px] shrink-0 bg-muted/20 border-r border-border flex items-start px-6 pt-6 pb-6">
    <div class="space-y-3">
      <slot name="left" />
    </div>
  </div>
  
  <!-- Right Column + Bottom: Flexible with sticky bottom -->
  <div class="flex-1 flex flex-col">
    <!-- Right Column: User Interactions (scrollable if needed) -->
    <div class="flex-1 flex items-center justify-center p-6 overflow-y-auto">
      <slot name="right" />
    </div>
    
    <!-- Bottom Action Section (sticky to right column) -->
    <div class="shrink-0 flex items-center justify-end p-6 bg-muted/10 border-t border-black/10 dark:border-white/20">
      <slot name="action" />
    </div>
  </div>
</div>

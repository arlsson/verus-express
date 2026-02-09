<!-- 
  Component: SecurityStep
  Purpose: Security guidelines step before showing seed phrase
  Last Updated: Simplified to content only, checkbox moved to main WalletCreation component
  Security: Ensures users read security guidelines before accessing sensitive data
-->

<script lang="ts">
  // Props (none needed for display-only component)
</script>

<!-- Security Guidelines Content -->
<div class="w-full max-w-md mx-auto">
  
  <!-- Security Guidelines -->
  <div class="p-6 border-l-4 border-amber-500 bg-amber-50 dark:bg-amber-950/20 rounded-r-lg">
    <h3 class="text-foreground font-semibold text-lg mb-4">Security guidelines</h3>
    
    <div class="space-y-3 text-foreground text-sm">
      <p>• Write these words on paper in exact order</p>
      <p>• Never save screenshots or digital copies</p>
      <p>• Store your backup in a secure location</p>
      <p>• Never share these words with anyone</p>
    </div>
  </div>
  
</div>

<!-- 
  Component: NameStep
  Purpose: Content-only for wallet naming step (right side content)
  Last Updated: Fixed centering and picker functionality
  Security: No sensitive data - only wallet customization
-->

<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  
  // Props
  let { 
    walletData = { name: '', emoji: '💰', color: 'blue', password: '' },
    onUpdate = (data: object) => {},
    errorMessage = ''
  } = $props();
  
  // Local state
  let showEmojiPicker = $state(false);
  let showColorPicker = $state(false);
  
  // Local validation
  const hasInvalidChars = $derived(/[/\\:*?"<>|]/.test(walletData.name));
  
  // Text constants
  const invalidCharsMessage = 'Name cannot contain special characters: / \\ : * ? " < > |';
  
  // Better emoji options (money, crypto, identity themed)
  const emojiOptions = [
    '💰', '💎', '🪙', '🔐', '⚡', '🔥', 
    '🚀', '🌟', '🛡️', '🔑', '👑', '⭐'
  ];
  
  // Extended color palette (grouped by color family)
  const colorOptions = [
    // Blues
    { name: 'blue', class: 'bg-blue-500 dark:bg-blue-600' },
    { name: 'indigo', class: 'bg-indigo-500 dark:bg-indigo-600' },
    { name: 'sky', class: 'bg-sky-500 dark:bg-sky-600' },
    { name: 'cyan', class: 'bg-cyan-500 dark:bg-cyan-600' },
    
    // Greens  
    { name: 'green', class: 'bg-green-500 dark:bg-green-600' },
    { name: 'emerald', class: 'bg-emerald-500 dark:bg-emerald-600' },
    { name: 'teal', class: 'bg-teal-500 dark:bg-teal-600' },
    { name: 'lime', class: 'bg-lime-500 dark:bg-lime-600' },
    
    // Warm colors
    { name: 'red', class: 'bg-red-500 dark:bg-red-600' },
    { name: 'orange', class: 'bg-orange-500 dark:bg-orange-600' },
    { name: 'amber', class: 'bg-amber-500 dark:bg-amber-600' },
    { name: 'yellow', class: 'bg-yellow-500 dark:bg-yellow-600' },
    
    // Purples & pinks
    { name: 'purple', class: 'bg-purple-500 dark:bg-purple-600' },
    { name: 'violet', class: 'bg-violet-500 dark:bg-violet-600' },
    { name: 'pink', class: 'bg-pink-500 dark:bg-pink-600' },
    { name: 'rose', class: 'bg-rose-500 dark:bg-rose-600' },
    
    // Neutrals
    { name: 'slate', class: 'bg-slate-500 dark:bg-slate-600' },
    { name: 'gray', class: 'bg-gray-500 dark:bg-gray-600' },
    { name: 'zinc', class: 'bg-zinc-500 dark:bg-zinc-600' },
    { name: 'stone', class: 'bg-stone-500 dark:bg-stone-600' }
  ];
  
  const selectedColorClass = $derived(colorOptions.find(c => c.name === walletData.color)?.class || colorOptions[0].class);
</script>

<!-- Content centered in available space -->
<div class="flex flex-col items-center justify-center min-h-full py-8">
  <div class="text-center space-y-8 w-full max-w-lg">
    
    <!-- Main Icon with Controls -->
    <div class="relative">
      <!-- Main Icon Display -->
      <div class="{selectedColorClass} w-32 h-32 rounded-3xl flex items-center justify-center shadow-xl border-4 border-white/20 mx-auto">
        <span class="text-6xl" role="img">{walletData.emoji}</span>
      </div>
      
      <!-- Small Picker Controls -->
      <div class="absolute right-24 top-1/2 -translate-y-1/2 flex flex-col gap-2">
        <!-- Emoji Picker -->
        <button
          onclick={() => { showEmojiPicker = !showEmojiPicker; showColorPicker = false; }}
          class="w-8 h-8 rounded-full bg-background border-2 border-border hover:border-primary hover:scale-105 transition-all flex items-center justify-center shadow-md"
          title="Change emoji"
        >
          <span class="text-sm" role="img">{walletData.emoji}</span>
        </button>
        
        <!-- Color Picker -->
        <button
          onclick={() => { showColorPicker = !showColorPicker; showEmojiPicker = false; }}
          class="w-8 h-8 rounded-full bg-background border-2 border-border hover:border-primary hover:scale-105 transition-all flex items-center justify-center shadow-md"
          title="Change color"
        >
          <div class="{selectedColorClass} w-4 h-4 rounded-full"></div>
        </button>
      </div>
    </div>
    
    <!-- Large Name Input -->
    <div>
      <input
        type="text"
        value={walletData.name}
        oninput={(e) => onUpdate({ name: (e.target as HTMLInputElement).value })}
        placeholder="Choose a name"
        class="w-full text-2xl font-bold text-center bg-background/60 border-2 border-border rounded-2xl px-8 py-6 
               focus:border-primary focus:bg-background focus:shadow-lg transition-all outline-none
               placeholder:text-muted-foreground/40 {hasInvalidChars ? 'text-destructive border-destructive' : 'text-card-foreground'}"
        autocomplete="off"
        spellcheck="false"
        autocorrect="off"
        autocapitalize="off"
      />
    </div>
    
    <!-- Validation Messages -->
    {#if hasInvalidChars}
      <div class="bg-destructive/10 border border-destructive/20 rounded-lg p-3">
        <p class="text-xs text-destructive">{invalidCharsMessage}</p>
      </div>
    {:else if walletData.name.length > 50}
      <div class="bg-destructive/10 border border-destructive/20 rounded-lg p-3">
        <p class="text-xs text-destructive">Name must be 50 characters or less</p>
      </div>
    {/if}
    
    <!-- Error Message -->
    {#if errorMessage}
      <div class="bg-destructive/10 border border-destructive/20 rounded-lg p-3">
        <p class="text-destructive text-xs">{errorMessage}</p>
      </div>
    {/if}
  </div>
</div>

<!-- Fixed Overlay Pickers -->
{#if showEmojiPicker}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/20" onclick={() => showEmojiPicker = false}>
    <div class="bg-background border border-border rounded-2xl p-4 shadow-2xl" onclick={(e) => e.stopPropagation()}>
      <div class="grid grid-cols-6 gap-3">
        {#each emojiOptions as emoji}
          <button
            onclick={() => { onUpdate({ emoji }); showEmojiPicker = false; }}
            class="w-14 h-14 rounded-xl hover:bg-muted transition-colors flex items-center justify-center
                   {emoji === walletData.emoji ? 'bg-primary/10 ring-2 ring-primary' : ''}"
          >
            <span class="text-2xl" role="img">{emoji}</span>
          </button>
        {/each}
      </div>
    </div>
  </div>
{/if}

{#if showColorPicker}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/20" onclick={() => showColorPicker = false}>
    <div class="bg-background border border-border rounded-2xl p-4 shadow-2xl" onclick={(e) => e.stopPropagation()}>
      <div class="grid grid-cols-10 gap-2">
        {#each colorOptions as color}
          <button
            onclick={() => { onUpdate({ color: color.name }); showColorPicker = false; }}
            class="w-8 h-8 rounded-full hover:scale-110 transition-all duration-200 
                   {color.name === walletData.color ? 'ring-2 ring-white scale-110' : ''}"
            title={color.name}
          >
            <div class="{color.class} w-full h-full rounded-full shadow-lg"></div>
          </button>
        {/each}
      </div>
    </div>
  </div>
{/if}
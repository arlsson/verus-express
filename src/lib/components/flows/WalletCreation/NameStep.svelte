<!-- 
  Component: NameStep
  Purpose: Content-only for wallet naming step (right side content).
  Last Updated: Explicitly typed picker items in template.
  Security: No sensitive data - only wallet customization.
-->

<script lang="ts">
  import { i18nStore } from '$lib/i18n';
  import { Input } from '$lib/components/ui/input';
  import { badgeVariants } from '$lib/components/ui/badge';
  import { cn } from '$lib/utils.js';

  type WalletData = {
    name: string;
    emoji: string;
    color: string;
    password: string;
    network: 'mainnet' | 'testnet';
  };

  type ColorOption = {
    name: string;
    class: string;
  };

  // Props
  let {
    walletData = { name: '', emoji: '💰', color: 'blue', password: '', network: 'mainnet' },
    onUpdate = (_data: Partial<WalletData>) => {},
    errorMessage = ''
  }: {
    walletData: WalletData;
    onUpdate: (data: Partial<WalletData>) => void;
    errorMessage: string;
  } = $props();

  const i18n = $derived($i18nStore);

  // Local state
  let showEmojiPicker = $state(false);
  let showColorPicker = $state(false);

  // Local validation
  const hasInvalidChars = $derived(/[/\\:*?"<>|]/.test(walletData.name));

  // Better emoji options (money, crypto, identity themed)
  const emojiOptions: string[] = ['💰', '💎', '🪙', '🔐', '⚡', '🔥', '🚀', '🌟', '🛡️', '🔑', '👑', '⭐'];

  // Extended color palette (grouped by color family)
  const colorOptions: ColorOption[] = [
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

  const selectedColorClass = $derived(
    colorOptions.find((c) => c.name === walletData.color)?.class || colorOptions[0].class
  );
</script>

<!-- Content centered in available space -->
<div class="flex min-h-full flex-col items-center justify-center py-4">
  <div class="w-full max-w-lg space-y-6 text-center">
    <!-- Main Icon with Controls -->
    <div class="relative">
      <!-- Main Icon Display -->
      <div
        class="{selectedColorClass} mx-auto flex h-28 w-28 items-center justify-center rounded-3xl border-4 border-white/20 shadow-xl"
      >
        <span class="text-6xl" role="img">{walletData.emoji}</span>
      </div>

      <!-- Small Picker Controls -->
      <div class="absolute left-1/2 top-1/2 ml-20 flex -translate-y-1/2 flex-col gap-2">
        <!-- Emoji Picker -->
        <button
          onclick={() => {
            showEmojiPicker = !showEmojiPicker;
            showColorPicker = false;
          }}
          class="w-8 h-8 rounded-full bg-background border-2 border-border hover:border-primary hover:scale-105 transition-all flex items-center justify-center shadow-md"
          title={i18n.t('walletCreation.name.changeEmoji')}
          aria-label={i18n.t('walletCreation.name.changeEmoji')}
        >
          <span class="text-sm" role="img">{walletData.emoji}</span>
        </button>

        <!-- Color Picker -->
        <button
          onclick={() => {
            showColorPicker = !showColorPicker;
            showEmojiPicker = false;
          }}
          class="w-8 h-8 rounded-full bg-background border-2 border-border hover:border-primary hover:scale-105 transition-all flex items-center justify-center shadow-md"
          title={i18n.t('walletCreation.name.changeColor')}
          aria-label={i18n.t('walletCreation.name.changeColor')}
        >
          <div class="{selectedColorClass} w-4 h-4 rounded-full"></div>
        </button>
      </div>
    </div>

    <!-- Name Input -->
    <div class="mx-auto w-full max-w-[420px]">
      <Input
        type="text"
        value={walletData.name}
        oninput={(e) => onUpdate({ name: (e.target as HTMLInputElement).value })}
        placeholder={i18n.t('walletCreation.name.placeholder')}
        variant="lg"
        class="w-full bg-background/60 text-center placeholder:text-muted-foreground/40
               focus-visible:bg-background {hasInvalidChars ? 'text-destructive' : 'text-card-foreground'}"
        autocomplete="off"
        spellcheck="false"
        autocorrect="off"
        autocapitalize="off"
      />
    </div>

    <!-- Network Selection -->
    <div class="space-y-2">
      <div class="flex items-center justify-center gap-2">
        <button
          type="button"
          onclick={() => onUpdate({ network: 'mainnet' })}
          class={cn(
            badgeVariants({ variant: walletData.network === 'mainnet' ? 'default' : 'outline' }),
            'h-6 cursor-pointer px-2.5 text-[11px] font-semibold transition-colors hover:bg-accent/60'
          )}
        >
          {i18n.t('walletCreation.name.mainnetTitle')}
        </button>
        <button
          type="button"
          onclick={() => onUpdate({ network: 'testnet' })}
          class={cn(
            badgeVariants({ variant: walletData.network === 'testnet' ? 'default' : 'outline' }),
            'h-6 cursor-pointer px-2.5 text-[11px] font-semibold transition-colors hover:bg-accent/60'
          )}
        >
          {i18n.t('walletCreation.name.testnetTitle')}
        </button>
      </div>
      {#if walletData.network === 'testnet'}
        <p class="text-xs text-amber-700 dark:text-amber-400">
          {i18n.t('walletCreation.name.testnetWarning')}
        </p>
      {/if}
    </div>

    <!-- Validation Messages -->
    {#if hasInvalidChars}
      <div class="bg-destructive/10 border border-destructive/20 rounded-lg p-3">
        <p class="text-xs text-destructive">{i18n.t('walletCreation.name.invalidChars')}</p>
      </div>
    {:else if walletData.name.length > 16}
      <div class="bg-destructive/10 border border-destructive/20 rounded-lg p-3">
        <p class="text-xs text-destructive">{i18n.t('walletCreation.name.maxLength')}</p>
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
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/20"
    role="button"
    tabindex="0"
    aria-label={i18n.t('walletCreation.name.closeEmojiPicker')}
    onclick={(event) => {
      if (event.currentTarget === event.target) {
        showEmojiPicker = false;
      }
    }}
    onkeydown={(event) => {
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        showEmojiPicker = false;
      }
      if (event.key === 'Escape') {
        showEmojiPicker = false;
      }
    }}
  >
    <div
      class="bg-background border border-border rounded-2xl p-4 shadow-2xl"
      role="dialog"
      aria-modal="true"
      aria-label={i18n.t('walletCreation.name.emojiPicker')}
    >
      <div class="grid grid-cols-6 gap-3">
        {#each emojiOptions as emoji}
          {@const emojiValue = emoji as string}
          <button
            onclick={() => {
              onUpdate({ emoji: emojiValue });
              showEmojiPicker = false;
            }}
            class="w-14 h-14 rounded-xl hover:bg-muted transition-colors flex items-center justify-center
                   {emojiValue === walletData.emoji ? 'bg-primary/10 ring-2 ring-primary' : ''}"
          >
            <span class="text-2xl" role="img">{emojiValue}</span>
          </button>
        {/each}
      </div>
    </div>
  </div>
{/if}

{#if showColorPicker}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/20"
    role="button"
    tabindex="0"
    aria-label={i18n.t('walletCreation.name.closeColorPicker')}
    onclick={(event) => {
      if (event.currentTarget === event.target) {
        showColorPicker = false;
      }
    }}
    onkeydown={(event) => {
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        showColorPicker = false;
      }
      if (event.key === 'Escape') {
        showColorPicker = false;
      }
    }}
  >
    <div
      class="bg-background border border-border rounded-2xl p-4 shadow-2xl"
      role="dialog"
      aria-modal="true"
      aria-label={i18n.t('walletCreation.name.colorPicker')}
    >
      <div class="grid grid-cols-10 gap-2">
        {#each colorOptions as color}
          {@const colorOption = color as ColorOption}
          <button
            onclick={() => {
              onUpdate({ color: colorOption.name });
              showColorPicker = false;
            }}
            class="w-8 h-8 rounded-full hover:scale-110 transition-all duration-200 
                   {colorOption.name === walletData.color ? 'ring-2 ring-white scale-110' : ''}"
            title={colorOption.name}
            aria-label={i18n.t('walletCreation.name.selectColor', { color: colorOption.name })}
          >
            <div class="{colorOption.class} w-full h-full rounded-full shadow-lg"></div>
          </button>
        {/each}
      </div>
    </div>
  </div>
{/if}

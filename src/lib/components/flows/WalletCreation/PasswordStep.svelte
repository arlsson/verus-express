<!-- 
  Component: PasswordStep
  Purpose: Complete password step with 2-column layout + bottom action
  Last Updated: Created for wallet encryption password
  Security: Handles wallet password - cleared after use
-->

<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { invoke } from '@tauri-apps/api/core';
  
  // Props
  let { 
    walletData = { name: '', emoji: '💰', color: 'blue', password: '' },
    onUpdate = (data: object) => {},
    onNext = () => {},
    seedPhrase = '',
    onWalletCreated = () => {}
  } = $props();
  
  // Local state
  let confirmPassword = $state('');
  let isLoading = $state(false);
  let errorMessage = $state('');
  
  // Password validation
  const passwordsMatch = $derived(walletData.password === confirmPassword && walletData.password !== '');
  const passwordValid = $derived(walletData.password.length >= 8);
  const canCreateWallet = $derived(passwordValid && passwordsMatch);
  
  async function createWallet() {
    if (!canCreateWallet) return;
    
    isLoading = true;
    errorMessage = '';
    
    try {
      console.info('[WALLET] Creating wallet:', walletData.name);
      
      const result = await invoke('create_wallet', {
        request: {
          wallet_name: walletData.name,
          seed_phrase: seedPhrase
        },
        password: walletData.password
      }) as { wallet_id: string; success: boolean };
      
      console.info('[WALLET] Wallet created successfully:', result.wallet_id);
      onWalletCreated();
      
    } catch (error) {
      console.error('[WALLET] Wallet creation failed:', error);
      errorMessage = 'Failed to create wallet. Please try again.';
    } finally {
      isLoading = false;
    }
  }
</script>

<!-- Content only for password step -->
<div class="space-y-5 max-w-sm mx-auto">
  
  <!-- Password Input -->
  <div class="space-y-2">
    <label for="wallet-password" class="text-sm font-medium text-card-foreground">
      Choose Password
    </label>
    <Input
      id="wallet-password"
      type="password"
      value={walletData.password}
      oninput={(e) => onUpdate({ password: (e.target as HTMLInputElement).value })}
      placeholder="Enter password"
      autocomplete="new-password"
    />
    <p class="text-xs text-muted-foreground">
      Minimum 8 characters
    </p>
  </div>
  
  <!-- Confirm Password -->
  <div class="space-y-2">
    <label for="confirm-password" class="text-sm font-medium text-card-foreground">
      Confirm Password
    </label>
    <Input
      id="confirm-password"
      type="password"
      value={confirmPassword}
      oninput={(e) => { confirmPassword = (e.target as HTMLInputElement).value; }}
      placeholder="Confirm password"
      autocomplete="new-password"
      class={confirmPassword && !passwordsMatch ? 'border-destructive' : ''}
    />
    {#if confirmPassword && !passwordsMatch}
      <p class="text-xs text-destructive">Passwords don't match</p>
    {/if}
  </div>
  
  <!-- Password Info -->
  <div class="bg-muted/50 border border-border rounded-lg p-3">
    <h4 class="text-card-foreground font-semibold text-sm mb-1">Password Security</h4>
    <p class="text-xs text-muted-foreground">
      This password encrypts your wallet on this device only. It's different from your recovery phrase.
    </p>
  </div>
  
  <!-- Error Message -->
  {#if errorMessage}
    <div class="bg-destructive/10 border border-destructive/20 rounded-lg p-3">
      <p class="text-destructive text-xs text-center">{errorMessage}</p>
    </div>
  {/if}
</div>

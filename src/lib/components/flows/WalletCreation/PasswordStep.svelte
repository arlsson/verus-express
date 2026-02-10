<!--
  Component: PasswordStep
  Purpose: Password validation input for wallet creation flow
  Last Updated: Removed duplicate create_wallet invoke path
  Security: Handles wallet password input only; parent component performs creation
-->

<script lang="ts">
  import { Input } from '$lib/components/ui/input';
  import { getPasswordStrength, MIN_PASS_SCORE, scorePassword } from '$lib/utils/auth/scorePassword';

  type WalletData = {
    name: string;
    emoji: string;
    color: string;
    password: string;
    network: 'mainnet' | 'testnet';
  };

  type PasswordStepProps = {
    walletData?: WalletData;
    onUpdate?: any;
    onCanCreateChanged?: any;
  };

  // Props
  /* eslint-disable prefer-const */
  let {
    walletData = { name: '', emoji: '💰', color: 'blue', password: '', network: 'mainnet' },
    onUpdate = () => {},
    onCanCreateChanged = () => {}
  }: PasswordStepProps = $props();
  /* eslint-enable prefer-const */

  // Local state
  let confirmPassword = $state('');

  // Password strength tracking
  let passwordScore = $state(0);
  const passwordStrength = $derived(getPasswordStrength(passwordScore));
  const strengthBars = [0, 1, 2, 3, 4];

  // Update score when password changes
  $effect(() => {
    if (!walletData.password) {
      passwordScore = 0;
    } else {
      passwordScore = scorePassword(walletData.password);
    }
  });

  // Password validation with strength requirements
  const passwordsMatch = $derived(walletData.password === confirmPassword && walletData.password !== '');
  const passwordValid = $derived(passwordScore >= MIN_PASS_SCORE);
  const canCreateWallet = $derived(passwordValid && passwordsMatch);

  // Notify parent component of validation state
  $effect(() => {
    onCanCreateChanged(canCreateWallet);
  });
</script>

<!-- Content only for password step -->
<div class="space-y-5 max-w-sm mx-auto">
  <!-- Password Input with Strength Indicator -->
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
      class={walletData.password && !passwordValid ? 'border-destructive' : ''}
    />

    <!-- Strength Bars (5 bars) -->
    {#if walletData.password}
      <div class="flex gap-1 h-1.5">
        {#each strengthBars as i}
          {@const isActive = i < passwordStrength.level}
          {@const barColor = isActive
            ? passwordStrength.color === 'destructive'
              ? 'bg-destructive'
              : passwordStrength.color === 'blue-500'
                ? 'bg-blue-500'
                : passwordStrength.color === 'primary'
                  ? 'bg-primary'
                  : passwordStrength.color === 'green-500'
                    ? 'bg-green-500 dark:bg-green-600'
                    : 'bg-muted'
            : 'bg-muted'}
          <div class="flex-1 rounded-full transition-colors {barColor}"></div>
        {/each}
      </div>

      <!-- Strength Label -->
      {@const labelColor = passwordStrength.color === 'destructive'
        ? 'text-destructive'
        : passwordStrength.color === 'blue-500'
          ? 'text-blue-500'
          : passwordStrength.color === 'primary'
            ? 'text-primary'
            : passwordStrength.color === 'green-500'
              ? 'text-green-500 dark:text-green-400'
              : 'text-muted-foreground'}
      <p class="text-xs {labelColor}">
        Strength: {passwordStrength.label}
      </p>
    {/if}

    <p class="text-xs text-muted-foreground">
      Minimum 7 characters with good variety (letters, numbers, symbols)
    </p>

    <!-- Password Strength Error -->
    {#if walletData.password && !passwordValid}
      <div class="bg-destructive/10 border border-destructive/20 rounded-lg p-2">
        <p class="text-xs text-destructive">Please enter a stronger password</p>
      </div>
    {/if}
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
      oninput={(e) => {
        confirmPassword = (e.target as HTMLInputElement).value;
      }}
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
</div>

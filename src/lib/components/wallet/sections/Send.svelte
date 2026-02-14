<!--
  Component: Send
  Purpose: Send flow — form -> preflight -> confirm -> send (preflight_id only). Uses txMachine.
  Last Updated: Module 9 — txService, txMachine, PreflightResult display
  Security: No tx hex or signing data; send by preflight_id only
-->

<script lang="ts">
  import { useMachine } from '@xstate/svelte';
  import { txMachine } from '$lib/machines/txMachine.js';
  import { coinsStore } from '$lib/stores/coins.js';
  import { transactionStore } from '$lib/stores/transactions.js';
  import { walletChannelsStore } from '$lib/stores/walletChannels.js';
  import { channelIdForCoin } from '$lib/utils/channelId.js';
  import * as walletService from '$lib/services/walletService.js';
  import * as Card from '$lib/components/ui/card';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import SendIcon from '@lucide/svelte/icons/send';
  import { i18nStore } from '$lib/i18n';
  import type { PreflightParams } from '$lib/types/wallet.js';
  import { resolveCoinPresentation } from '$lib/coins/presentation.js';
  import CoinIcon from '$lib/components/wallet/CoinIcon.svelte';

  const { snapshot: txSnapshot, send } = useMachine(txMachine);
  const coins = $derived($coinsStore);
  const i18n = $derived($i18nStore);

  const sendableCoins = $derived(
    coins.filter(
      (c) =>
        c.compatibleChannels.includes('vrpc') ||
        c.compatibleChannels.includes('btc') ||
        c.compatibleChannels.includes('eth') ||
        c.compatibleChannels.includes('erc20')
    )
  );
  const sendableCoinOptions = $derived(
    sendableCoins.map((coin) => {
      const presentation = resolveCoinPresentation(coin);
      return {
        coin,
        displayTicker: presentation.displayTicker,
        displayName: presentation.displayName,
      };
    })
  );

  let selectedCoinId = $state('VRSC');
  let toAddress = $state('');
  let amount = $state('');
  let memo = $state('');

  $effect(() => {
    if (sendableCoins.length === 0) return;
    if (!sendableCoins.some((c) => c.id === selectedCoinId)) {
      selectedCoinId = sendableCoins[0].id;
    }
  });

  const selectedCoin = $derived(sendableCoins.find((c) => c.id === selectedCoinId) ?? sendableCoins[0]);
  const selectedCoinPresentation = $derived(
    selectedCoin ? resolveCoinPresentation(selectedCoin) : null
  );
  const walletChannels = $derived($walletChannelsStore);
  const channelId = $derived(
    selectedCoin
      ? walletChannels.byCoinId[selectedCoin.id] ??
          channelIdForCoin(selectedCoin, walletChannels.vrpcAddress ?? undefined)
      : null
  );

  const value = $derived($txSnapshot?.value);
  const context = $derived($txSnapshot?.context ?? null);
  const preflightResult = $derived(context?.preflightResult ?? null);
  const sendResult = $derived(context?.sendResult ?? null);
  const error = $derived(context?.error ?? null);

  function handleSubmit() {
    if (!channelId || !toAddress.trim() || !amount.trim()) return;
    const params: PreflightParams = {
      coinId: selectedCoin!.id,
      channelId,
      toAddress: toAddress.trim(),
      amount: amount.trim(),
      memo: memo.trim() || undefined
    };
    send({ type: 'SUBMIT_FORM', params });
  }

  function handleConfirm() {
    send({ type: 'CONFIRM' });
  }

  function handleReset() {
    send({ type: 'RESET' });
    toAddress = '';
    amount = '';
    memo = '';
  }

  async function refreshTxHistory() {
    if (!channelId) return;
    try {
      const txs = await walletService.getTransactionHistory(channelId);
      transactionStore.update((m) => ({ ...m, [channelId]: txs }));
    } catch {
      // ignore
    }
  }
</script>

<div class="flex flex-col gap-6 p-6 max-w-lg mx-auto">
  {#if value === 'idle' || value === 'preflighting'}
    <Card.Root>
      <Card.Header>
        <Card.Title>{i18n.t('wallet.send.title')}</Card.Title>
        <Card.Description>{i18n.t('wallet.send.description')}</Card.Description>
      </Card.Header>
      <Card.Content class="space-y-4">
        <div>
          <label for="send-coin" class="text-sm font-medium mb-2 block">{i18n.t('wallet.send.coinLabel')}</label>
          {#if selectedCoin && selectedCoinPresentation}
            <div class="mb-2 flex items-center gap-2 rounded-md border border-border/70 bg-muted/20 px-2 py-1.5">
              <CoinIcon
                coinId={selectedCoin.id}
                coinName={selectedCoinPresentation.displayName}
                proto={selectedCoin.proto}
                size={24}
                showBadge
                decorative
              />
              <p class="text-xs font-medium text-foreground">
                {selectedCoinPresentation.displayTicker} - {selectedCoinPresentation.displayName}
              </p>
            </div>
          {/if}
          <select
            id="send-coin"
            class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm"
            bind:value={selectedCoinId}
            disabled={value === 'preflighting'}
          >
            {#each sendableCoinOptions as option}
              <option value={option.coin.id}>{option.displayTicker} - {option.displayName}</option>
            {/each}
          </select>
        </div>
        <div>
          <label for="send-to" class="text-sm font-medium mb-2 block">{i18n.t('wallet.send.toAddressLabel')}</label>
          <Input
            id="send-to"
            type="text"
            placeholder={i18n.t('wallet.send.toAddressPlaceholder')}
            bind:value={toAddress}
            disabled={value === 'preflighting'}
          />
        </div>
        <div>
          <label for="send-amount" class="text-sm font-medium mb-2 block">{i18n.t('wallet.send.amountLabel')}</label>
          <Input
            id="send-amount"
            type="text"
            placeholder={i18n.t('wallet.send.amountPlaceholder')}
            bind:value={amount}
            disabled={value === 'preflighting'}
          />
        </div>
        <div>
          <label for="send-memo" class="text-sm font-medium mb-2 block">{i18n.t('wallet.send.memoLabel')}</label>
          <Input
            id="send-memo"
            type="text"
            placeholder={i18n.t('wallet.send.memoPlaceholder')}
            bind:value={memo}
            disabled={value === 'preflighting'}
          />
        </div>
        {#if value === 'preflighting'}
          <p class="text-sm text-muted-foreground">{i18n.t('wallet.send.preparing')}</p>
        {/if}
        <Button class="w-full" onclick={handleSubmit} disabled={!toAddress.trim() || !amount.trim() || value === 'preflighting'}>
          <SendIcon class="h-4 w-4 mr-2" />
          {i18n.t('wallet.send.continue')}
        </Button>
      </Card.Content>
    </Card.Root>
  {:else if value === 'confirming' && preflightResult}
    <Card.Root>
      <Card.Header>
        <Card.Title>{i18n.t('wallet.send.confirmTitle')}</Card.Title>
        <Card.Description>{i18n.t('wallet.send.confirmDescription')}</Card.Description>
      </Card.Header>
      <Card.Content class="space-y-3">
        <div class="text-sm">
          <p><span class="text-muted-foreground">{i18n.t('wallet.send.to')}</span> {preflightResult.toAddress}</p>
          <p><span class="text-muted-foreground">{i18n.t('wallet.send.amount')}</span> {preflightResult.value}</p>
          <p><span class="text-muted-foreground">{i18n.t('wallet.send.fee')}</span> {preflightResult.fee} {preflightResult.feeCurrency}</p>
          {#if preflightResult.feeTakenMessage}
            <p class="text-muted-foreground text-xs">{preflightResult.feeTakenMessage}</p>
          {/if}
          {#each preflightResult.warnings as w}
            <p class="text-amber-600 dark:text-amber-400 text-xs">{w.message}</p>
          {/each}
        </div>
        <div class="flex gap-2">
          <Button variant="outline" class="flex-1" onclick={() => send({ type: 'RESET' })}>{i18n.t('common.back')}</Button>
          <Button class="flex-1" onclick={handleConfirm}>{i18n.t('common.confirm')}</Button>
        </div>
      </Card.Content>
    </Card.Root>
  {:else if value === 'sending'}
    <Card.Root>
      <Card.Content class="py-8 text-center">
        <p class="text-muted-foreground">{i18n.t('wallet.send.sending')}</p>
      </Card.Content>
    </Card.Root>
  {:else if value === 'success' && sendResult}
    <Card.Root>
      <Card.Header>
        <Card.Title>{i18n.t('wallet.send.sentTitle')}</Card.Title>
        <Card.Description>{i18n.t('wallet.send.sentDescription')}</Card.Description>
      </Card.Header>
      <Card.Content class="space-y-3">
        <p class="text-sm font-mono break-all">{sendResult.txid}</p>
        <p class="text-sm text-muted-foreground">
          {i18n.t('wallet.send.sentSummary', { value: sendResult.value, address: sendResult.toAddress })}
        </p>
        <Button class="w-full" onclick={() => { handleReset(); refreshTxHistory(); }}>
          {i18n.t('common.done')}
        </Button>
      </Card.Content>
    </Card.Root>
  {:else if value === 'error'}
    <Card.Root>
      <Card.Content class="py-6">
        <p class="text-destructive mb-4">{error}</p>
        <Button onclick={handleReset}>{i18n.t('common.retry')}</Button>
      </Card.Content>
    </Card.Root>
  {:else}
    <Card.Root>
      <Card.Content class="py-8 text-center">
        <SendIcon class="h-12 w-12 text-muted-foreground mx-auto mb-4" />
        <h2 class="text-xl font-semibold mb-2">{i18n.t('wallet.send.title')}</h2>
        <p class="text-muted-foreground text-sm">{i18n.t('wallet.send.emptyDescription')}</p>
      </Card.Content>
    </Card.Root>
  {/if}
</div>

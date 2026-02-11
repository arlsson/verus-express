<!-- 
  Component: AppSidebar
  Purpose: Left sidebar navigation for wallet sections
  Last Updated: Sidebar redesign with hybrid functional/dummy navigation
  Security: No sensitive operations - navigation only
-->

<script lang="ts">
  import * as Sidebar from '$lib/components/ui/sidebar';
  import HomeIcon from '@lucide/svelte/icons/home';
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
  import CircleIcon from '@lucide/svelte/icons/circle';
  import LockIcon from '@lucide/svelte/icons/lock';
  import UserIcon from '@lucide/svelte/icons/user';
  import RefreshCwIcon from '@lucide/svelte/icons/refresh-cw';
  import BookOpenIcon from '@lucide/svelte/icons/book-open';
  import SendIcon from '@lucide/svelte/icons/send';
  import DownloadIcon from '@lucide/svelte/icons/download';
  import { i18nStore } from '$lib/i18n';

  type SectionId = 'overview' | 'send' | 'receive' | 'conversions' | 'identity' | 'address-book';
  type PlaceholderItem =
    | 'verus'
    | 'ethereum'
    | 'bitcoin'
    | 'identity-max'
    | 'identity-business'
    | 'identity-varrr-max'
    | 'activity';

  interface FunctionalMenuItem {
    id: SectionId;
    title: string;
    icon: typeof HomeIcon;
  }

  // `activeSection` is bindable, so this props object must be mutable.
  /* eslint-disable prefer-const */
  let { activeSection = $bindable('overview' as SectionId) }: { activeSection?: SectionId } = $props();
  /* eslint-enable prefer-const */

  let isWalletOpen = $state(true);
  let isIdentitiesOpen = $state(true);
  let isVrscOpen = $state(true);
  let isVarrrOpen = $state(true);
  let isMoreOpen = $state(false);
  let selectedPlaceholder = $state<PlaceholderItem>('verus');

  const i18n = $derived($i18nStore);
  const functionalMenuItems = $derived<FunctionalMenuItem[]>([
    { id: 'send', title: i18n.t('wallet.sidebar.send'), icon: SendIcon },
    { id: 'receive', title: i18n.t('wallet.sidebar.receive'), icon: DownloadIcon },
    { id: 'conversions', title: i18n.t('wallet.sidebar.conversions'), icon: RefreshCwIcon },
    { id: 'identity', title: i18n.t('wallet.sidebar.identity'), icon: UserIcon },
    { id: 'address-book', title: i18n.t('wallet.sidebar.addressBook'), icon: BookOpenIcon }
  ]);

  $effect(() => {
    if (activeSection !== 'overview' && !isMoreOpen) {
      isMoreOpen = true;
    }
  });

  function selectPlaceholder(item: PlaceholderItem): void {
    selectedPlaceholder = item;
  }
</script>

<Sidebar.Root>
  <Sidebar.Content class="pt-11">
    <Sidebar.Group>
      <Sidebar.GroupContent>
        <Sidebar.Menu>
          <Sidebar.MenuItem>
            <Sidebar.MenuButton
              size="sm"
              class="h-8 rounded-md px-2 text-[13px]"
              isActive={activeSection === 'overview'}
              onclick={() => (activeSection = 'overview')}
              tooltipContent={i18n.t('wallet.sidebar.dashboard')}
            >
              <HomeIcon class="size-3.5" />
              <span>{i18n.t('wallet.sidebar.dashboard')}</span>
            </Sidebar.MenuButton>
          </Sidebar.MenuItem>

          <Sidebar.MenuItem>
            <Sidebar.MenuButton
              size="sm"
              class="h-8 rounded-md px-2 text-[13px]"
              onclick={() => (isWalletOpen = !isWalletOpen)}
              tooltipContent={i18n.t('wallet.sidebar.wallet')}
            >
              <BookOpenIcon class="size-3.5" />
              <span>{i18n.t('wallet.sidebar.wallet')}</span>
              <ChevronDownIcon
                class={'ml-auto size-3.5 text-current transition-transform duration-200 ' +
                  (isWalletOpen ? 'rotate-180' : '')}
              />
            </Sidebar.MenuButton>
            {#if isWalletOpen}
              <Sidebar.MenuSub class="mx-2.5 gap-0.5 border-sidebar-border/70 px-2 py-0.5">
                <Sidebar.MenuSubItem>
                  <Sidebar.MenuSubButton
                    href="#"
                    size="sm"
                    class="h-6.5 px-1.5 text-[12.5px]"
                    isActive={selectedPlaceholder === 'verus'}
                    onclick={(event) => {
                      event.preventDefault();
                      selectPlaceholder('verus');
                    }}
                  >
                    <CircleIcon class="size-2 fill-current stroke-0 text-blue-500" />
                    <span>{i18n.t('wallet.sidebar.verus')}</span>
                  </Sidebar.MenuSubButton>
                </Sidebar.MenuSubItem>
                <Sidebar.MenuSubItem>
                  <Sidebar.MenuSubButton
                    href="#"
                    size="sm"
                    class="h-6.5 px-1.5 text-[12.5px]"
                    isActive={selectedPlaceholder === 'ethereum'}
                    onclick={(event) => {
                      event.preventDefault();
                      selectPlaceholder('ethereum');
                    }}
                  >
                    <CircleIcon class="size-2 fill-current stroke-0 text-sky-500" />
                    <span>{i18n.t('wallet.sidebar.ethereum')}</span>
                  </Sidebar.MenuSubButton>
                </Sidebar.MenuSubItem>
                <Sidebar.MenuSubItem>
                  <Sidebar.MenuSubButton
                    href="#"
                    size="sm"
                    class="h-6.5 px-1.5 text-[12.5px]"
                    isActive={selectedPlaceholder === 'bitcoin'}
                    onclick={(event) => {
                      event.preventDefault();
                      selectPlaceholder('bitcoin');
                    }}
                  >
                    <CircleIcon class="size-2 fill-current stroke-0 text-amber-500" />
                    <span>{i18n.t('wallet.sidebar.bitcoin')}</span>
                  </Sidebar.MenuSubButton>
                </Sidebar.MenuSubItem>
              </Sidebar.MenuSub>
            {/if}
          </Sidebar.MenuItem>

          <Sidebar.MenuItem>
            <Sidebar.MenuButton
              size="sm"
              class="h-8 rounded-md px-2 text-[13px]"
              onclick={() => (isIdentitiesOpen = !isIdentitiesOpen)}
              tooltipContent={i18n.t('wallet.sidebar.identities')}
            >
              <UserIcon class="size-3.5" />
              <span>{i18n.t('wallet.sidebar.identities')}</span>
              <ChevronDownIcon
                class={'ml-auto size-3.5 text-current transition-transform duration-200 ' +
                  (isIdentitiesOpen ? 'rotate-180' : '')}
              />
            </Sidebar.MenuButton>

            {#if isIdentitiesOpen}
              <Sidebar.MenuSub class="mx-2.5 gap-0.5 border-sidebar-border/70 px-2 py-0.5">
                <Sidebar.MenuSubItem>
                  <Sidebar.MenuSubButton
                    href="#"
                    size="sm"
                    class="h-6.5 px-1.5 text-[12.5px]"
                    onclick={(event) => {
                      event.preventDefault();
                      isVrscOpen = !isVrscOpen;
                    }}
                  >
                    <ChevronDownIcon
                      class={'size-3 text-current transition-transform duration-200 ' +
                        (isVrscOpen ? 'rotate-180' : '')}
                    />
                    <span>{i18n.t('wallet.sidebar.vrsc')}</span>
                  </Sidebar.MenuSubButton>
                </Sidebar.MenuSubItem>

                {#if isVrscOpen}
                  <Sidebar.MenuSub class="ms-1.5 mx-1.5 gap-0.5 border-sidebar-border/60 px-1.5 py-0.5">
                    <Sidebar.MenuSubItem>
                      <Sidebar.MenuSubButton
                        href="#"
                        size="sm"
                        class="h-6.5 px-1.5 text-[12.5px]"
                        isActive={selectedPlaceholder === 'identity-max'}
                        onclick={(event) => {
                          event.preventDefault();
                          selectPlaceholder('identity-max');
                        }}
                      >
                        <CircleIcon class="size-2 fill-current stroke-0 text-indigo-500" />
                        <span>{i18n.t('wallet.sidebar.identityMax')}</span>
                      </Sidebar.MenuSubButton>
                    </Sidebar.MenuSubItem>
                    <Sidebar.MenuSubItem>
                      <Sidebar.MenuSubButton
                        href="#"
                        size="sm"
                        class="h-6.5 px-1.5 text-[12.5px]"
                        isActive={selectedPlaceholder === 'identity-business'}
                        onclick={(event) => {
                          event.preventDefault();
                          selectPlaceholder('identity-business');
                        }}
                      >
                        <CircleIcon class="size-2 fill-current stroke-0 text-violet-500" />
                        <span>{i18n.t('wallet.sidebar.identityBusiness')}</span>
                      </Sidebar.MenuSubButton>
                    </Sidebar.MenuSubItem>
                  </Sidebar.MenuSub>
                {/if}

                <Sidebar.MenuSubItem>
                  <Sidebar.MenuSubButton
                    href="#"
                    size="sm"
                    class="h-6.5 px-1.5 text-[12.5px]"
                    onclick={(event) => {
                      event.preventDefault();
                      isVarrrOpen = !isVarrrOpen;
                    }}
                  >
                    <ChevronDownIcon
                      class={'size-3 text-current transition-transform duration-200 ' +
                        (isVarrrOpen ? 'rotate-180' : '')}
                    />
                    <span>{i18n.t('wallet.sidebar.varrr')}</span>
                  </Sidebar.MenuSubButton>
                </Sidebar.MenuSubItem>

                {#if isVarrrOpen}
                  <Sidebar.MenuSub class="ms-1.5 mx-1.5 gap-0.5 border-sidebar-border/60 px-1.5 py-0.5">
                    <Sidebar.MenuSubItem>
                      <Sidebar.MenuSubButton
                        href="#"
                        size="sm"
                        class="h-6.5 px-1.5 text-[12.5px]"
                        isActive={selectedPlaceholder === 'identity-varrr-max'}
                        onclick={(event) => {
                          event.preventDefault();
                          selectPlaceholder('identity-varrr-max');
                        }}
                      >
                        <CircleIcon class="size-2 fill-current stroke-0 text-indigo-500" />
                        <span>{i18n.t('wallet.sidebar.identityMax')}</span>
                        <LockIcon class="ml-auto size-3 text-current" />
                      </Sidebar.MenuSubButton>
                    </Sidebar.MenuSubItem>
                  </Sidebar.MenuSub>
                {/if}

                <Sidebar.MenuSubItem>
                  <Sidebar.MenuSubButton
                    href="#"
                    size="sm"
                    class="h-6.5 px-1.5 text-[12.5px]"
                    onclick={(event) => {
                      event.preventDefault();
                    }}
                  >
                    <span class="text-sm leading-none text-primary">+</span>
                    <span>{i18n.t('wallet.sidebar.createNew')}</span>
                  </Sidebar.MenuSubButton>
                </Sidebar.MenuSubItem>
              </Sidebar.MenuSub>
            {/if}
          </Sidebar.MenuItem>

          <Sidebar.MenuItem>
            <Sidebar.MenuButton
              size="sm"
              class="h-8 rounded-md px-2 text-[13px]"
              isActive={selectedPlaceholder === 'activity'}
              onclick={() => selectPlaceholder('activity')}
              tooltipContent={i18n.t('wallet.sidebar.activity')}
            >
              <RefreshCwIcon class="size-3.5" />
              <span>{i18n.t('wallet.sidebar.activity')}</span>
            </Sidebar.MenuButton>
          </Sidebar.MenuItem>

          <Sidebar.MenuItem>
            <Sidebar.MenuButton
              size="sm"
              class="h-8 rounded-md px-2 text-[13px]"
              isActive={activeSection !== 'overview'}
              onclick={() => (isMoreOpen = !isMoreOpen)}
              tooltipContent={i18n.t('wallet.sidebar.more')}
            >
              <BookOpenIcon class="size-3.5" />
              <span>{i18n.t('wallet.sidebar.more')}</span>
              <ChevronDownIcon
                class={'ml-auto size-3.5 text-current transition-transform duration-200 ' +
                  (isMoreOpen ? 'rotate-180' : '')}
              />
            </Sidebar.MenuButton>
            {#if isMoreOpen}
              <Sidebar.MenuSub class="mx-2.5 gap-0.5 border-sidebar-border/70 px-2 py-0.5">
                {#each functionalMenuItems as item}
                  <Sidebar.MenuSubItem>
                    <Sidebar.MenuSubButton
                      href="#"
                      size="sm"
                      class="h-6.5 px-1.5 text-[12.5px]"
                      isActive={activeSection === item.id}
                      onclick={(event) => {
                        event.preventDefault();
                        activeSection = item.id;
                      }}
                    >
                      <item.icon class="size-3.5" />
                      <span>{item.title}</span>
                    </Sidebar.MenuSubButton>
                  </Sidebar.MenuSubItem>
                {/each}
              </Sidebar.MenuSub>
            {/if}
          </Sidebar.MenuItem>
        </Sidebar.Menu>
      </Sidebar.GroupContent>
    </Sidebar.Group>
  </Sidebar.Content>

  <Sidebar.Footer>
    <Sidebar.Separator />
    <Sidebar.Menu>
      <Sidebar.MenuItem>
        <Sidebar.MenuButton
          size="sm"
          class="h-8 rounded-lg border border-sidebar-border/60 bg-sidebar-accent/30 px-2 text-[13px]"
        >
          <LockIcon class="size-3.5" />
          <span class="font-medium">{i18n.t('wallet.sidebar.idGuard')}</span>
        </Sidebar.MenuButton>
      </Sidebar.MenuItem>
    </Sidebar.Menu>
  </Sidebar.Footer>
</Sidebar.Root>

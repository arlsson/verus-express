<!-- 
  Component: AppSidebar
  Purpose: Left sidebar navigation for wallet sections
  Last Updated: Initial creation
  Security: No sensitive operations - navigation only
-->

<script lang="ts">
  import * as Sidebar from '$lib/components/ui/sidebar';
  import HomeIcon from '@lucide/svelte/icons/home';
  import SendIcon from '@lucide/svelte/icons/send';
  import DownloadIcon from '@lucide/svelte/icons/download';
  import RefreshCwIcon from '@lucide/svelte/icons/refresh-cw';
  import UserIcon from '@lucide/svelte/icons/user';
  import BookOpenIcon from '@lucide/svelte/icons/book-open';
  import { i18nStore } from '$lib/i18n';

  interface MenuItem {
    id: string;
    title: string;
    icon: typeof HomeIcon;
  }

  const i18n = $derived($i18nStore);
  const menuItems = $derived<MenuItem[]>([
    { id: 'overview', title: i18n.t('wallet.sidebar.overview'), icon: HomeIcon },
    { id: 'send', title: i18n.t('wallet.sidebar.send'), icon: SendIcon },
    { id: 'receive', title: i18n.t('wallet.sidebar.receive'), icon: DownloadIcon },
    { id: 'conversions', title: i18n.t('wallet.sidebar.conversions'), icon: RefreshCwIcon },
    { id: 'identity', title: i18n.t('wallet.sidebar.identity'), icon: UserIcon },
    { id: 'address-book', title: i18n.t('wallet.sidebar.addressBook'), icon: BookOpenIcon },
  ]);

  let { activeSection = $bindable('overview') } = $props();
</script>

<Sidebar.Root>
  <Sidebar.Content>
    <Sidebar.Group>
      <Sidebar.GroupContent>
        <Sidebar.Menu>
          {#each menuItems as item}
            <Sidebar.MenuItem>
              <Sidebar.MenuButton
                isActive={activeSection === item.id}
                onclick={() => (activeSection = item.id)}
              >
                <item.icon />
                <span>{item.title}</span>
              </Sidebar.MenuButton>
            </Sidebar.MenuItem>
          {/each}
        </Sidebar.Menu>
      </Sidebar.GroupContent>
    </Sidebar.Group>
  </Sidebar.Content>
</Sidebar.Root>

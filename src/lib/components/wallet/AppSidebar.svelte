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

  interface MenuItem {
    id: string;
    title: string;
    icon: typeof HomeIcon;
  }

  const menuItems: MenuItem[] = [
    { id: 'overview', title: 'Overview', icon: HomeIcon },
    { id: 'send', title: 'Send', icon: SendIcon },
    { id: 'receive', title: 'Receive', icon: DownloadIcon },
    { id: 'conversions', title: 'Conversions', icon: RefreshCwIcon },
    { id: 'identity', title: 'Identity', icon: UserIcon },
    { id: 'address-book', title: 'Address Book', icon: BookOpenIcon },
  ];

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

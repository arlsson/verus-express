<!--
  Component: HelpSidebar
  Purpose: Shows slide-in help content with optional accordion sections and a community link.
  Last Updated: Rendered sections with headings as shadcn-style accordion items; intro text (no heading) stays as plain paragraph.
  Security: Display-only help content, no sensitive data.
-->

<script lang="ts">
  import * as Sheet from '$lib/components/ui/sheet';
  import * as Accordion from '$lib/components/ui/accordion';
  import { Separator } from '$lib/components/ui/separator';
  import { Button } from '$lib/components/ui/button';
  import { i18nStore } from '$lib/i18n';

  type HelpSection = {
    heading?: string;
    text: string;
  };

  interface HelpContent {
    sections: Array<HelpSection>;
  }

  let {
    isOpen = $bindable(false),
    title,
    content
  }: {
    isOpen?: boolean;
    title: string;
    content: HelpContent;
  } = $props();

  const sections = $derived(content.sections);
  const introSections = $derived(sections.filter((s) => !s.heading));
  const accordionSections = $derived(sections.filter((s): s is HelpSection & { heading: string } => !!s.heading));
  const i18n = $derived($i18nStore);
</script>

<Sheet.Root bind:open={isOpen}>
  <Sheet.Content class="w-[300px] max-w-[300px] p-6">
    {#snippet children()}
      <div class="flex h-full flex-col">
        <!-- Header -->
        <div class="mb-6">
          <h2 class="text-foreground text-lg font-semibold">
            {title}
          </h2>
        </div>

        <!-- Intro text (sections without heading) -->
        {#if introSections.length > 0}
          <div class="text-muted-foreground mb-4 space-y-2 text-left text-xs leading-relaxed">
            {#each introSections as section (section)}
              <p>{section.text}</p>
            {/each}
          </div>
        {/if}

        <!-- Accordion (sections with heading) -->
        {#if accordionSections.length > 0}
          <Accordion.Root type="single" class="flex-1 text-left">
            {#each accordionSections as section, i (section.heading)}
              <Accordion.Item value="section-{i}">
                <Accordion.Trigger>
                  {section.heading}
                </Accordion.Trigger>
                <Accordion.Content>
                  <p>{section.text}</p>
                </Accordion.Content>
              </Accordion.Item>
            {/each}
          </Accordion.Root>
        {/if}

        <!-- Footer -->
        <div class="mt-6 space-y-4">
          <Separator />
          <Button
            variant="outline"
            size="lg"
            onclick={() => window.open('https://discord.gg/VRSC', '_blank')}
            class="w-full justify-start border-[#5865F2]/35 text-sm text-[#5865F2] hover:bg-[#5865F2]/10 hover:text-[#4752C4] dark:border-[#5865F2]/45 dark:text-[#5865F2] dark:hover:bg-[#5865F2]/20 dark:hover:text-[#7B86F8]"
          >
            <svg class="h-4 w-4" viewBox="0 0 24 24" fill="currentColor">
              <path d="M20.317 4.3698a19.7913 19.7913 0 00-4.8851-1.5152.0741.0741 0 00-.0785.0371c-.211.3753-.4447.8648-.6083 1.2495-1.8447-.2762-3.68-.2762-5.4868 0-.1636-.3933-.4058-.8742-.6177-1.2495a.077.077 0 00-.0785-.037 19.7363 19.7363 0 00-4.8852 1.515.0699.0699 0 00-.0321.0277C.5334 9.0458-.319 13.5799.0992 18.0578a.0824.0824 0 00.0312.0561c2.0528 1.5076 4.0413 2.4228 5.9929 3.0294a.0777.0777 0 00.0842-.0276c.4616-.6304.8731-1.2952 1.226-1.9942a.076.076 0 00-.0416-.1057c-.6528-.2476-1.2743-.5495-1.8722-.8923a.077.077 0 01-.0076-.1277c.1258-.0943.2517-.1923.3718-.2914a.0743.0743 0 01.0776-.0105c3.9278 1.7933 8.18 1.7933 12.0614 0a.0739.0739 0 01.0785.0095c.1202.099.246.1981.3728.2924a.077.077 0 01-.0066.1276 12.2986 12.2986 0 01-1.873.8914.0766.0766 0 00-.0407.1067c.3604.698.7719 1.3628 1.225 1.9932a.076.076 0 00.0842.0286c1.961-.6067 3.9495-1.5219 6.0023-3.0294a.077.077 0 00.0313-.0552c.5004-5.177-.8382-9.6739-3.5485-13.6604a.061.061 0 00-.0312-.0286zM8.02 15.3312c-1.1825 0-2.1569-1.0857-2.1569-2.419 0-1.3332.9555-2.4189 2.157-2.4189 1.2108 0 2.1757 1.0952 2.1568 2.419-.0002 1.3332-.9555 2.4189-2.1569 2.4189zm7.9748 0c-1.1825 0-2.1569-1.0857-2.1569-2.419 0-1.3332.9554-2.4189 2.1569-2.4189 1.2108 0 2.1757 1.0952 2.1568 2.419 0 1.3332-.9554 2.4189-2.1568 2.4189Z" />
            </svg>
            {i18n.t('help.communityHangout')}
          </Button>
        </div>
      </div>
    {/snippet}
  </Sheet.Content>
</Sheet.Root>

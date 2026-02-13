<!--
  Component: HelpSidebar
  Purpose: Shows topic-based help in a right sheet with list and detail views.
  Security: Display-only help content, no sensitive data.
-->

<script lang="ts">
  import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import * as Accordion from '$lib/components/ui/accordion';
  import { Separator } from '$lib/components/ui/separator';
  import { Button } from '$lib/components/ui/button';
  import StandardRightSheet from '$lib/components/common/StandardRightSheet.svelte';
  import { i18nStore } from '$lib/i18n';

  type HelpQa = {
    id: string;
    question: string;
    answer: string;
  };

  type HelpTopic = {
    id: string;
    label: string;
    title: string;
    qas: Array<HelpQa>;
  };

  type HelpContent = {
    topics: Array<HelpTopic>;
  };

  /* eslint-disable prefer-const */
  let {
    isOpen = $bindable(false),
    title,
    content
  }: {
    isOpen?: boolean;
    title: string;
    content: HelpContent;
  } = $props();
  /* eslint-enable prefer-const */

  const topics = $derived(content.topics);
  const i18n = $derived($i18nStore);

  let view = $state<'topics' | 'detail'>('topics');
  let activeTopicId = $state<string | null>(null);
  let activeAccordionItem = $state('');

  const activeTopic = $derived(
    activeTopicId ? topics.find((topic) => topic.id === activeTopicId) ?? null : null
  );

  $effect(() => {
    if (isOpen) return;
    view = 'topics';
    activeTopicId = null;
    activeAccordionItem = '';
  });

  $effect(() => {
    if (view !== 'detail' || !activeTopic || activeTopic.qas.length === 0) return;
    if (activeTopic.qas.some((qa) => qa.id === activeAccordionItem)) return;
    activeAccordionItem = activeTopic.qas[0].id;
  });

  function openTopic(topicId: string) {
    const topic = topics.find((item) => item.id === topicId);
    if (!topic) return;
    activeTopicId = topic.id;
    activeAccordionItem = topic.qas[0]?.id ?? '';
    view = 'detail';
  }

  function backToTopics() {
    view = 'topics';
    activeTopicId = null;
    activeAccordionItem = '';
  }
</script>

<StandardRightSheet
  bind:isOpen
  {title}
  hideTitle={view === 'detail'}
  bodyClass={view === 'detail' ? 'mt-2' : ''}
>
  {#if view === 'topics'}
    <div class="flex h-full min-h-0 flex-col">
      <div class="flex-1 overflow-y-auto pr-1">
        <div class="divide-border/70 overflow-hidden rounded-md border border-border/70 divide-y">
          {#each topics as topic (topic.id)}
            <button
              type="button"
              class="group hover:bg-muted/45 focus-visible:ring-ring/50 text-foreground flex w-full items-center justify-between gap-3 px-3 py-3 text-left text-sm font-medium transition-colors outline-none focus-visible:ring-[2px]"
              onclick={() => openTopic(topic.id)}
            >
              <span>{topic.label}</span>
              <ChevronRightIcon
                class="text-muted-foreground size-4 shrink-0 transition-transform duration-150 group-hover:translate-x-0.5"
                aria-hidden="true"
              />
            </button>
          {/each}
        </div>
      </div>

      <div class="mt-6 space-y-4">
        <Separator />
        <Button
          variant="outline"
          size="lg"
          onclick={() => window.open('https://discord.gg/VRSC', '_blank')}
          class="w-full justify-center border-0 bg-[#5865F2]/10 text-sm text-[#4752C4] hover:bg-[#5865F2]/18 hover:text-[#3F49B7] dark:bg-[#5865F2]/20 dark:text-[#7B86F8] dark:hover:bg-[#5865F2]/30 dark:hover:text-[#A0A8FF]"
        >
          <svg class="h-4 w-4" viewBox="0 0 24 24" fill="currentColor">
            <path d="M20.317 4.3698a19.7913 19.7913 0 00-4.8851-1.5152.0741.0741 0 00-.0785.0371c-.211.3753-.4447.8648-.6083 1.2495-1.8447-.2762-3.68-.2762-5.4868 0-.1636-.3933-.4058-.8742-.6177-1.2495a.077.077 0 00-.0785-.037 19.7363 19.7363 0 00-4.8852 1.515.0699.0699 0 00-.0321.0277C.5334 9.0458-.319 13.5799.0992 18.0578a.0824.0824 0 00.0312.0561c2.0528 1.5076 4.0413 2.4228 5.9929 3.0294a.0777.0777 0 00.0842-.0276c.4616-.6304.8731-1.2952 1.226-1.9942a.076.076 0 00-.0416-.1057c-.6528-.2476-1.2743-.5495-1.8722-.8923a.077.077 0 01-.0076-.1277c.1258-.0943.2517-.1923.3718-.2914a.0743.0743 0 01.0776-.0105c3.9278 1.7933 8.18 1.7933 12.0614 0a.0739.0739 0 01.0785.0095c.1202.099.246.1981.3728.2924a.077.077 0 01-.0066.1276 12.2986 12.2986 0 01-1.873.8914.0766.0766 0 00-.0407.1067c.3604.698.7719 1.3628 1.225 1.9932a.076.076 0 00.0842.0286c1.961-.6067 3.9495-1.5219 6.0023-3.0294a.077.077 0 00.0313-.0552c.5004-5.177-.8382-9.6739-3.5485-13.6604a.061.061 0 00-.0312-.0286zM8.02 15.3312c-1.1825 0-2.1569-1.0857-2.1569-2.419 0-1.3332.9555-2.4189 2.157-2.4189 1.2108 0 2.1757 1.0952 2.1568 2.419-.0002 1.3332-.9555 2.4189-2.1569 2.4189zm7.9748 0c-1.1825 0-2.1569-1.0857-2.1569-2.419 0-1.3332.9554-2.4189 2.1569-2.4189 1.2108 0 2.1757 1.0952 2.1568 2.419 0 1.3332-.9554 2.4189-2.1568 2.4189Z" />
          </svg>
          {i18n.t('help.communityHangout')}
        </Button>
      </div>
    </div>
  {:else if activeTopic}
    <div class="flex h-full min-h-0 flex-col">
      <button
        type="button"
        class="text-muted-foreground hover:text-foreground mb-2 inline-flex items-center gap-1.5 text-sm transition-colors"
        onclick={backToTopics}
      >
        <ArrowLeftIcon class="size-4" />
        {i18n.t('common.back')}
      </button>

      <Accordion.Root
        type="single"
        class="mt-1 flex-1 overflow-y-auto pr-1 text-left"
        value={activeAccordionItem}
        onValueChange={(value) => {
          activeAccordionItem = value || '';
        }}
      >
        {#each activeTopic.qas as qa (qa.id)}
          <Accordion.Item value={qa.id}>
            <Accordion.Trigger>{qa.question}</Accordion.Trigger>
            <Accordion.Content>
              <p>{qa.answer}</p>
            </Accordion.Content>
          </Accordion.Item>
        {/each}
      </Accordion.Root>
    </div>
  {/if}
</StandardRightSheet>

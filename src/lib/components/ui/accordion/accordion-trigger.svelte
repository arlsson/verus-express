<!--
  Trigger wrapper for bits-ui Accordion. Adds flex layout and chevron icon.
-->
<script lang="ts">
	import { Accordion as AccordionPrimitive } from 'bits-ui';
	import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
	import { cn } from '$lib/utils.js';

	let {
		ref = $bindable(null),
		class: className,
		children,
		...restProps
	}: AccordionPrimitive.TriggerProps = $props();
</script>

<AccordionPrimitive.Trigger
	bind:ref
	data-slot="accordion-trigger"
	class={cn(
		'text-card-foreground hover:bg-muted/40 focus-visible:ring-ring/50 flex w-full items-center justify-between gap-3 rounded-md px-3 py-3 text-sm font-medium transition-all outline-none focus-visible:ring-[2px] [&[data-state=open]>svg]:rotate-180',
		className
	)}
	{...restProps}
>
	{#snippet children()}
		<span class="text-left">{@render children?.()}</span>
		<ChevronDownIcon class="text-muted-foreground size-4 shrink-0 transition-transform duration-200" />
	{/snippet}
</AccordionPrimitive.Trigger>

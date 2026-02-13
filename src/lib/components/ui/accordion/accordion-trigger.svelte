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
		'text-foreground focus-visible:ring-ring/50 flex w-full flex-1 items-start justify-between gap-4 py-4 text-sm font-medium transition-all outline-none hover:underline focus-visible:ring-[2px] [&[data-state=open]>svg]:rotate-180',
		className
	)}
	{...restProps}
>
	{#snippet children()}
		<span class="text-left">{@render children?.()}</span>
		<ChevronDownIcon class="text-muted-foreground size-4 shrink-0 transition-transform duration-200" />
	{/snippet}
</AccordionPrimitive.Trigger>

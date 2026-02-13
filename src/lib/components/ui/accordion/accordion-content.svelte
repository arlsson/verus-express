<!--
  Content wrapper for bits-ui Accordion. Adds accordion animation and padding.
-->
<script lang="ts">
	import { Accordion as AccordionPrimitive } from 'bits-ui';
	import { cn, type WithoutChild } from '$lib/utils.js';

	let {
		ref = $bindable(null),
		class: className,
		children,
		...restProps
	}: WithoutChild<AccordionPrimitive.ContentProps> = $props();
</script>

<AccordionPrimitive.Content
	bind:ref
	data-slot="accordion-content"
	class="overflow-hidden text-sm data-[state=closed]:animate-accordion-up data-[state=open]:animate-accordion-down"
	{...restProps}
>
	{#snippet children()}
		<div class={cn('text-muted-foreground pb-4 pt-0 text-sm leading-relaxed', className)}>
			<div>
				{@render children?.()}
			</div>
		</div>
	{/snippet}
</AccordionPrimitive.Content>

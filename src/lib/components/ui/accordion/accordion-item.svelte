<!--
  Item wrapper for bits-ui Accordion. Forwards props and optional styling.
-->
<script lang="ts">
	import { Accordion as AccordionPrimitive } from 'bits-ui';
	import { cn } from '$lib/utils.js';

	let {
		ref = $bindable(null),
		class: className,
		value,
		...restProps
	}: AccordionPrimitive.ItemProps = $props();
</script>

<AccordionPrimitive.Item
	bind:ref
	data-slot="accordion-item"
	class={cn(
		'border-border border-b last:border-b-0',
		className
	)}
	{value}
	{...restProps}
/>

<script lang="ts">
	import { ScrollArea as ScrollAreaPrimitive } from "bits-ui";
	import { cn, type WithElementRef } from "$lib/utils.js";
	import ScrollAreaThumb from "./scroll-area-thumb.svelte";

	let {
		ref = $bindable(null),
		orientation = "vertical",
		class: className,
		children,
		...restProps
	}: WithElementRef<ScrollAreaPrimitive.ScrollbarProps> = $props();
</script>

<ScrollAreaPrimitive.Scrollbar
	bind:ref
	data-slot="scroll-area-scrollbar"
	{orientation}
	class={cn(
		"flex touch-none select-none p-px transition-colors data-[orientation=vertical]:h-full data-[orientation=vertical]:w-2.5 data-[orientation=horizontal]:h-2.5 data-[orientation=horizontal]:flex-col",
		orientation === "vertical" ? "border-l border-l-transparent" : "border-t border-t-transparent",
		className
	)}
	{...restProps}
>
	{#if children}
		{@render children()}
	{:else}
		<ScrollAreaThumb />
	{/if}
</ScrollAreaPrimitive.Scrollbar>

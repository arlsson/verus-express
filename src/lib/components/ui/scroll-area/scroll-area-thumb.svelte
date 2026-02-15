<script lang="ts">
	import { ScrollArea as ScrollAreaPrimitive } from "bits-ui";
	import { cn, type WithElementRef } from "$lib/utils.js";

	let {
		ref = $bindable(null),
		class: className,
		...restProps
	}: WithElementRef<ScrollAreaPrimitive.ThumbProps> = $props();
</script>

<ScrollAreaPrimitive.Thumb
	bind:ref
	data-slot="scroll-area-thumb"
	class={cn("bg-foreground/20 relative flex-1 rounded-full", className)}
	{...restProps}
/>

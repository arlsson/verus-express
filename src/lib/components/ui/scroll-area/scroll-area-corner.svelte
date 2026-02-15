<script lang="ts">
	import { ScrollArea as ScrollAreaPrimitive } from "bits-ui";
	import { cn, type WithElementRef } from "$lib/utils.js";

	let {
		ref = $bindable(null),
		class: className,
		...restProps
	}: WithElementRef<ScrollAreaPrimitive.CornerProps> = $props();
</script>

<ScrollAreaPrimitive.Corner bind:ref data-slot="scroll-area-corner" class={cn("bg-border", className)} {...restProps} />

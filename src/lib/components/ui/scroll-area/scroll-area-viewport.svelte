<script lang="ts">
	import { ScrollArea as ScrollAreaPrimitive } from "bits-ui";
	import { cn, type WithElementRef } from "$lib/utils.js";

	let {
		ref = $bindable(null),
		class: className,
		children,
		...restProps
	}: WithElementRef<ScrollAreaPrimitive.ViewportProps> = $props();
</script>

<ScrollAreaPrimitive.Viewport
	bind:ref
	data-slot="scroll-area-viewport"
	class={cn("size-full rounded-[inherit]", className)}
	{...restProps}
>
	{@render children?.()}
</ScrollAreaPrimitive.Viewport>

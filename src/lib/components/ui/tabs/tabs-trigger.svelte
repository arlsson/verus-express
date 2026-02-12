<script lang="ts">
	import { Tabs as TabsPrimitive } from "bits-ui";
	import { cn } from "$lib/utils.js";

	let {
		ref = $bindable(null),
		class: className,
		children,
		...restProps
	}: TabsPrimitive.TriggerProps = $props();
</script>

<TabsPrimitive.Trigger
	bind:ref
	data-slot="tabs-trigger"
	class={cn(
		"focus-visible:ring-ring/50 inline-flex h-8 items-center justify-center rounded-sm px-3 text-sm font-medium whitespace-nowrap transition-colors outline-none focus-visible:ring-[3px] disabled:pointer-events-none disabled:opacity-50 data-[state=active]:bg-background data-[state=active]:text-foreground data-[state=active]:shadow-xs",
		className
	)}
	{...restProps}
>
	{@render children?.()}
</TabsPrimitive.Trigger>

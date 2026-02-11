<script lang="ts">
	import type { HTMLInputAttributes, HTMLInputTypeAttribute } from "svelte/elements";
	import EyeIcon from "@lucide/svelte/icons/eye";
	import EyeOffIcon from "@lucide/svelte/icons/eye-off";
	import { cn, type WithElementRef } from "$lib/utils.js";

	type InputType = Exclude<HTMLInputTypeAttribute, "file">;

	type Props = WithElementRef<
		Omit<HTMLInputAttributes, "type"> &
			({ type: "file"; files?: FileList } | { type?: InputType; files?: undefined })
	>;

	let {
		ref = $bindable(null),
		value = $bindable(),
		type,
		files = $bindable(),
		class: className,
		"data-slot": dataSlot = "input",
		...restProps
	}: Props = $props();

	let showPassword = $state(false);
</script>

{#if type === "file"}
	<input
		bind:this={ref}
		data-slot={dataSlot}
		class={cn(
			"selection:bg-primary selection:text-primary-foreground ring-offset-background placeholder:text-muted-foreground flex h-11 w-full min-w-0 rounded-md border border-transparent bg-transparent px-3 pt-1.5 text-sm font-medium shadow-none transition-[border-color,box-shadow] outline-none disabled:cursor-not-allowed disabled:opacity-50",
			"focus-visible:border-ring/70 focus-visible:ring-ring/50 focus-visible:ring-[3px]",
			"aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive",
			className
		)}
		type="file"
		bind:files
		bind:value
		{...restProps}
	/>
{:else}
	<div class="relative w-full">
		<input
			bind:this={ref}
			data-slot={dataSlot}
			class={cn(
				"bg-background/90 dark:bg-input/20 selection:bg-primary selection:text-primary-foreground ring-offset-background placeholder:text-muted-foreground flex h-11 w-full min-w-0 rounded-md border border-transparent px-4 py-2 text-base shadow-none transition-[border-color,box-shadow,background-color] outline-none disabled:cursor-not-allowed disabled:opacity-50 md:text-sm",
				"focus-visible:border-ring/70 focus-visible:ring-ring/50 focus-visible:ring-[3px]",
				"aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive",
				type === "password" && "pe-11",
				className
			)}
			type={type === "password" && showPassword ? "text" : type}
			bind:value
			{...restProps}
		/>
		{#if type === "password"}
			<button
				type="button"
				aria-label={showPassword ? "Hide password" : "Show password"}
				aria-pressed={showPassword}
				class="text-foreground/55 hover:text-foreground/80 focus-visible:ring-ring/50 absolute inset-y-0 end-0 inline-flex items-center justify-center px-3 outline-none focus-visible:ring-[2px]"
				onclick={() => {
					showPassword = !showPassword;
				}}
			>
				{#if showPassword}
					<EyeOffIcon class="size-4" />
				{:else}
					<EyeIcon class="size-4" />
				{/if}
			</button>
		{/if}
	</div>
{/if}

<script lang="ts">
	import { Select as SelectPrimitive } from 'bits-ui';
	import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
	import { cn, type WithoutChild } from '$lib/utils.js';

	let {
		ref = $bindable(null),
		class: className,
		children,
		size = 'default',
		...restProps
	}: WithoutChild<SelectPrimitive.TriggerProps> & {
		size?: 'sm' | 'default';
	} = $props();
</script>

<SelectPrimitive.Trigger
	bind:ref
	data-slot="select-trigger"
	data-size={size}
	class={cn(
		"flex h-9 w-full min-w-0 items-center justify-between gap-2 border border-line-strong bg-raised px-2.5 text-sm whitespace-nowrap text-ink transition-colors outline-none select-none hover:border-ink-muted hover:bg-raised focus-visible:border-ink focus-visible:ring-2 focus-visible:ring-ink/30 disabled:cursor-not-allowed disabled:opacity-50 aria-invalid:border-scarlet aria-invalid:ring-2 aria-invalid:ring-scarlet/20 data-placeholder:text-ink-muted [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg]:text-ink-muted [&_svg:not([class*='size-'])]:size-4",
		className
	)}
	{...restProps}
>
	{@render children?.()}
	<ChevronDownIcon class="pointer-events-none size-4 text-muted-foreground" />
</SelectPrimitive.Trigger>

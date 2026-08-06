<script lang="ts">
	import { Progress as ProgressPrimitive } from 'bits-ui';
	import { cn, type WithoutChildrenOrChild } from '$lib/utils.js';

	let {
		ref = $bindable(null),
		class: className,
		max = 100,
		value,
		...restProps
	}: WithoutChildrenOrChild<ProgressPrimitive.RootProps> = $props();
	let indicatorWidth = $derived.by(() => {
		const safeMax = Math.max(max ?? 100, 1);
		return Math.min(Math.max(((value ?? 0) / safeMax) * 100, 0), 100);
	});
</script>

<ProgressPrimitive.Root
	bind:ref
	data-slot="progress"
	class={cn('relative flex h-1 w-full items-center overflow-hidden bg-muted', className)}
	{value}
	{max}
	{...restProps}
>
	<svg
		class="block size-full"
		viewBox="0 0 100 1"
		preserveAspectRatio="none"
		aria-hidden="true"
		focusable="false"
	>
		<rect class="fill-primary" width={indicatorWidth} height="1" />
	</svg>
</ProgressPrimitive.Root>

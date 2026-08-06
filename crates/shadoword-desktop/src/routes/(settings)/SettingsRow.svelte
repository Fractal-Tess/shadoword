<script lang="ts">
	import { cn } from '$lib/utils';
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';

	let {
		children,
		class: className,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & { children: Snippet } = $props();
</script>

<div
	class={cn(
		'grid grid-cols-[minmax(0,1fr)_auto] items-center gap-6 px-4 py-[0.9rem]',
		'max-[800px]:grid-cols-1 max-[800px]:gap-[0.7rem]',
		className
	)}
	{...restProps}
>
	{@render children()}
</div>

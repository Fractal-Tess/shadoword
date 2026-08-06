<script lang="ts">
	import { cn } from '$lib/utils';
	import type { Snippet } from 'svelte';
	import type { ClassValue } from 'svelte/elements';

	let {
		id,
		title,
		description,
		icon,
		children,
		class: className
	}: {
		id: string;
		title: string;
		description: string;
		icon: Snippet;
		children: Snippet;
		class?: ClassValue;
	} = $props();
</script>

<section {id} class={cn('scroll-mt-4', className)}>
	<header class="mb-3 flex items-center gap-[0.7rem]">
		<div class="grid size-8 shrink-0 place-items-center border border-line text-ink-dim">
			{@render icon()}
		</div>
		<div class="min-w-0">
			<h2
				class="font-display text-[1.125rem] leading-none font-normal tracking-[0.035em] text-ink uppercase"
			>
				{title}
			</h2>
			<p class="mt-[0.2rem] text-[0.6875rem] leading-[1.45] text-ink-muted">
				{description}
			</p>
		</div>
	</header>
	{@render children()}
</section>

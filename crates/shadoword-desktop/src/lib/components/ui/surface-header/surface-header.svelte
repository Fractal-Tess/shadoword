<script lang="ts">
	import type { Snippet } from 'svelte';

	let { title, description, actions }: { title: string; description: string; actions?: Snippet } =
		$props();
</script>

<header
	class="flex items-start justify-between gap-8 border-b border-line pb-[1.1rem] [@media(max-width:720px)]:flex-col [@media(max-width:720px)]:items-stretch [@media(max-width:720px)]:gap-4"
>
	<div class="max-w-[46rem] min-w-0">
		<h1
			class="mr-[calc((var(--squeeze-heading)-1)*100%)] origin-left [transform:scaleX(var(--squeeze-heading))] font-display text-[clamp(2.125rem,3.4vw,3rem)] leading-[0.94] font-normal tracking-[-0.02em] text-ink uppercase"
		>
			{title}
		</h1>
		<span class="mt-[0.7rem] block max-w-[68ch] font-mono text-sm leading-[1.55] text-ink-muted">
			{description}
		</span>
	</div>
	{#if actions}
		<div class="flex shrink-0 items-center gap-[0.55rem] pt-[0.3rem]">{@render actions()}</div>
	{/if}
</header>

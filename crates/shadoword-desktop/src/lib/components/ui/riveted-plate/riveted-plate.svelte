<script lang="ts">
	import { cn } from '$lib/utils';
	import type { Snippet } from 'svelte';
	import type { ClassValue } from 'svelte/elements';

	let { children, class: className }: { children: Snippet; class?: ClassValue } = $props();
</script>

<div
	class={cn(
		'relative rounded-[var(--plate-radius)] border border-line-strong bg-plate shadow-rivet-plate',
		"[--rivet-pitch:16.66%] before:pointer-events-none before:absolute before:inset-x-[0.6rem] before:top-[0.4rem] before:h-[9px] before:bg-rivets before:[background-size:var(--rivet-pitch)_9px] before:bg-repeat-x before:content-['']",
		"after:pointer-events-none after:absolute after:inset-x-[0.6rem] after:bottom-[0.4rem] after:h-[9px] after:bg-rivets after:[background-size:var(--rivet-pitch)_9px] after:bg-repeat-x after:content-['']",
		'max-[999px]:rounded-none max-[999px]:border-line max-[999px]:bg-transparent max-[999px]:shadow-none max-[999px]:before:hidden max-[999px]:after:hidden',
		className
	)}
>
	{@render children()}
</div>

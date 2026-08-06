<script lang="ts">
	import { cn } from '$lib/utils';
	import type { Snippet } from 'svelte';
	import type { ClassValue } from 'svelte/elements';

	let { children, class: className }: { children: Snippet; class?: ClassValue } = $props();
</script>

<div class={cn('border border-line bg-plate [&>*+*]:border-t [&>*+*]:border-line', className)}>
	{@render children()}
</div>

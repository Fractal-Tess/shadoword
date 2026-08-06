<script lang="ts">
	import type { RuntimeState } from '$lib/types';
	import { cn } from '$lib/utils';

	/**
	 * Four runtime states in a world with one accent.
	 *
	 * The previous pill gave each state its own hue — blue, green, amber, red —
	 * which is the dashboard reflex and which this world forbids: an instrument
	 * lighting up in four colours has stopped telling you which light matters. But
	 * collapsing all four onto scarlet would make "the pool has an unhealthy unit"
	 * and "the runtime is gone" look identical, and that is a real regression on a
	 * surface whose job is to be read at a glance.
	 *
	 * State is differentiated by shape, motion, and a required text label.
	 */
	let {
		state = 'ready',
		label,
		compact = false,
		active = false
	}: { state?: RuntimeState; label: string; compact?: boolean; active?: boolean } = $props();

	let marked = $derived(active || state === 'warning' || state === 'offline');
	let glyphSurface = $derived(
		active || state === 'offline'
			? 'bg-scarlet shadow-[0_0_0_2.5px_color-mix(in_srgb,var(--scarlet),transparent_62%)]'
			: state === 'warning'
				? 'bg-transparent shadow-[inset_0_0_0_1.5px_var(--scarlet-lamp)]'
				: state === 'loading'
					? 'bg-ink-muted'
					: 'bg-ink'
	);
</script>

<span
	class={cn(
		'status inline-flex max-w-full min-w-0 items-center gap-2 font-mono text-[0.8125rem] text-ink-dim',
		compact && 'text-xs'
	)}
	data-state={state}
>
	<span
		class={cn(
			'relative size-2 shrink-0',
			glyphSurface,
			state === 'loading' &&
				'animate-[glyph-scan_0.9s_linear_infinite] [mask-image:linear-gradient(90deg,#000_0_3px,transparent_3px_4px)] [mask-size:4px_100%] [mask-repeat:repeat-x] motion-reduce:animate-none'
		)}
		aria-hidden="true"
	></span>
	<span
		class={cn('overflow-hidden text-ellipsis whitespace-nowrap', marked && 'text-scarlet-lamp')}
	>
		{label}
	</span>
</span>

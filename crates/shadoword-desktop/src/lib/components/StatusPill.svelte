<script lang="ts">
	import type { RuntimeState } from '$lib/types';

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
	 * So the differentiator is *shape*, which is free and which this world already
	 * has an opinion about:
	 *
	 *   ready    solid off-white square. The default. Nothing is wrong, so nothing
	 *            gets the accent.
	 *   loading  the same square in muted ink, scanning. Transient, so motion
	 *            carries it and no colour has to.
	 *   warning  a *hollow* scarlet-lamp square. Marked, not filled — attention,
	 *            not failure. Which is the world's rationing rule applied to a 7px
	 *            glyph: ink means marked, fill means active.
	 *   offline  a solid scarlet square in a scarlet halo. Filled, and given the
	 *            extra weight failure earns.
	 *
	 * The `label` prop is required, so shape and colour are never the only channel
	 * — they are what makes the label findable without reading it.
	 */
	let {
		state = 'ready',
		label,
		compact = false
	}: { state?: RuntimeState; label: string; compact?: boolean } = $props();
</script>

<span class:compact class="status" data-state={state}>
	<span class="glyph" aria-hidden="true"></span>
	<span class="label">{label}</span>
</span>

<style>
	.status {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		/* `fit-content` alone let the label push past its container: pool summaries run
		   to "Remote · 1 ready · 1 busy · 1 unhealthy", and in a 14rem rail that ran off
		   the window edge with no ellipsis, because a shrink-to-fit box has nothing to
		   shrink against. The max-width is what gives `.label`'s ellipsis a boundary. */
		max-width: 100%;
		width: fit-content;
		min-width: 0;
		color: var(--ink-dim);
		font-family: var(--font-mono);
		font-size: 0.75rem;
	}

	.label {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.glyph {
		position: relative;
		width: 0.4375rem;
		height: 0.4375rem;
		flex-shrink: 0;
		background: var(--ink);
	}

	/* A 1px gap sweeping across a 7px square. A spinner would be the app asking to
	   be watched while it waits; this is the readout admitting it is mid-update.
	   The gap is punched with a mask rather than painted with a pseudo-element, so
	   it needs no knowledge of the ground behind the pill — which matters, because
	   this pill appears on night, on a plate and on a selected row. */
	.status[data-state='loading'] .glyph {
		background: var(--ink-muted);
		mask-image: linear-gradient(90deg, #000 0 3px, transparent 3px 4px);
		mask-size: 4px 100%;
		mask-repeat: repeat-x;
		animation: glyph-scan 0.9s linear infinite;
	}

	/* Hollow, and ground-agnostic: the fill is transparent and an inset shadow
	   paints a 1px rim inwards, so the pill can sit on night, on a plate or on a
	   selected row without carrying a hard-coded centre colour. `border` would do
	   the same job but leaves a 4px hole in a 7px box that rounds unevenly at
	   fractional device pixels. */
	.status[data-state='warning'] .glyph {
		background: transparent;
		box-shadow: inset 0 0 0 1.5px var(--scarlet-lamp);
	}

	.status[data-state='warning'] .label {
		color: var(--scarlet-lamp);
	}

	.status[data-state='offline'] .glyph {
		background: var(--scarlet);
		box-shadow: 0 0 0 2.5px color-mix(in srgb, var(--scarlet), transparent 62%);
	}

	.status[data-state='offline'] .label {
		color: var(--scarlet-lamp);
	}

	.compact {
		font-size: 0.6875rem;
	}

	@keyframes glyph-scan {
		to {
			mask-position: 4px 0;
		}
	}
</style>

<script lang="ts">
	import type { Snippet } from 'svelte';

	/**
	 * Every view's head. The title is the display face — heavy condensed grotesque,
	 * uppercase, squeezed by the recorded factor — and the kicker above it is mono.
	 * That inversion is deliberate: the kicker names which panel of the machine you
	 * are on, so it belongs to the same small-caps mono family as every other
	 * legend in the window, while the title is the one place per view where the
	 * display cut gets to be large.
	 *
	 * The kicker is *not* scarlet. It was, and five views each carrying a scarlet
	 * label is scarlet meaning nothing on any of them — the accent has to stay
	 * reserved for what is live. A rule under the header carries the structure
	 * instead.
	 */
	let {
		kicker,
		title,
		description,
		actions
	}: { kicker: string; title: string; description: string; actions?: Snippet } = $props();
</script>

<header class="surface-header">
	<div class="heading-copy">
		<p class="mono-label">{kicker}</p>
		<h1 class="display-view">{title}</h1>
		<span class="mono-legend">{description}</span>
	</div>
	{#if actions}
		<div class="header-actions">{@render actions()}</div>
	{/if}
</header>

<style>
	.surface-header {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 2rem;
		margin-bottom: 1.25rem;
		border-bottom: 1px solid var(--line);
		padding-bottom: 1.1rem;
	}

	.heading-copy {
		min-width: 0;
		max-width: 46rem;
	}

	.mono-label {
		margin: 0 0 0.7rem;
	}

	/* The squeeze is a horizontal scale, so the collapsed glyphs leave
	   (1 - factor) of the box empty on the right while the layout still reserves
	   the full width. A negative right margin hands that slack back, which matters
	   because the title sits beside `.header-actions` and would otherwise push
	   them off a narrow window for space it is not using. */
	.display-view {
		color: var(--ink);
		margin-right: calc((var(--squeeze-heading) - 1) * 100%);
	}

	.heading-copy > span {
		display: block;
		max-width: 68ch;
		margin-top: 0.7rem;
		color: var(--ink-muted);
	}

	.header-actions {
		display: flex;
		align-items: center;
		gap: 0.55rem;
		flex-shrink: 0;
		padding-top: 0.3rem;
	}

	@media (max-width: 720px) {
		.surface-header {
			align-items: stretch;
			flex-direction: column;
			gap: 1rem;
		}
	}
</style>

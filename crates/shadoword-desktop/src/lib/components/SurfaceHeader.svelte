<script lang="ts">
	import type { Snippet } from 'svelte';

	let { title, description, actions }: { title: string; description: string; actions?: Snippet } =
		$props();
</script>

<header class="surface-header">
	<div class="heading-copy">
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

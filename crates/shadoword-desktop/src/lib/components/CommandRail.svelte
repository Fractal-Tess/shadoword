<script lang="ts">
	import BrandMark from '$lib/components/BrandMark.svelte';
	import EnvironmentSelector from '$lib/components/shell/EnvironmentSelector.svelte';
	import PrimaryNavigation from '$lib/components/shell/PrimaryNavigation.svelte';
	import RuntimeStation from '$lib/components/shell/RuntimeStation.svelte';
</script>

<aside class="command-rail">
	<div class="brand-station">
		<BrandMark />
		<p class="mono-micro">Local-first speech workspace</p>
	</div>
	<EnvironmentSelector />
	<PrimaryNavigation />
	<RuntimeStation />
</aside>

<style>
	.command-rail {
		display: flex;
		min-width: 0;
		flex-direction: column;
		border-right: 1px solid var(--line);
		background: var(--surface-1);
	}

	.brand-station {
		border-bottom: 1px solid var(--line);
		padding: 1rem 1.15rem 0.85rem;
	}

	.brand-station p {
		margin: 0.5rem 0 0;
		color: var(--ink-muted);
	}

	@media (max-width: 900px) {
		.brand-station {
			display: grid;
			justify-items: center;
			padding-inline: 0;
		}

		.brand-station :global(.display-wordmark),
		.brand-station p {
			display: none;
		}
	}
</style>

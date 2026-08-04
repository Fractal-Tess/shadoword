<script lang="ts">
	import { Box, Clock3, Info, Mic2, Settings2 } from '@lucide/svelte';
	import BrandMark from '$lib/components/BrandMark.svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import type { DesktopAppState } from '$lib/app-state.svelte';
	import { inferencePoolSummary } from '$lib/inference-pool';
	import type { PageId, RuntimeState } from '$lib/types';

	let { app, activePage = $bindable() }: { app: DesktopAppState; activePage: PageId } = $props();
	let mode = $derived(app.settings?.mode ?? 'remote');
	let poolStatus = $derived(app.overview?.status.inference_pool ?? null);
	let modelName = $derived.by(() => {
		const path = app.overview?.runtime.model_path;
		return app.overview?.models.find((model) => path?.endsWith(model.filename))?.name ?? 'No model';
	});
	let runtimeState = $derived<RuntimeState>(
		app.activity === 'busy'
			? 'loading'
			: (poolStatus?.unhealthy_units ?? 0) > 0
				? 'warning'
				: app.overview
					? 'ready'
					: 'offline'
	);

	const destinations = [
		{ id: 'transcribe', label: 'Transcribe', icon: Mic2 },
		{ id: 'models', label: 'Models', icon: Box },
		{ id: 'history', label: 'History', icon: Clock3 },
		{ id: 'settings', label: 'Settings', icon: Settings2 },
		{ id: 'about', label: 'About', icon: Info }
	] satisfies Array<{ id: PageId; label: string; icon: typeof Mic2 }>;
</script>

<aside class="command-rail">
	<div class="brand-station">
		<BrandMark />
		<p class="mono-micro">Private speech workspace</p>
	</div>

	<nav aria-label="Primary navigation">
		{#each destinations as destination (destination.id)}
			{@const Icon = destination.icon}
			<button
				type="button"
				class:active={activePage === destination.id}
				onclick={() => (activePage = destination.id)}
				aria-current={activePage === destination.id ? 'page' : undefined}
				aria-label={destination.label}
			>
				{#if activePage === destination.id}
					<!-- The selected destination is marked by a scarlet bar on the rail's
					     own edge, not by a scarlet label. Ink means marked, fill means
					     active — and a filled slab in the nav would compete with the one
					     scarlet fill in this window that means "recording right now". -->
					<span class="marker" aria-hidden="true"></span>
				{/if}
				<Icon size={16} strokeWidth={1.6} aria-hidden="true" />
				<span class="destination-label display-legend">{destination.label}</span>
			</button>
		{/each}
	</nav>

	<!--
		The window's one physical object, and its placement is the point: a real
		machine carries its identity plate at the bottom of its front panel, and this
		is what the machine *is* — which runtime, which model, which pool. Radius plus
		bevel plus shadow plus rivets belongs to this element and to nothing else in
		the app; a second would make the material a texture.
	-->
	<div class="runtime-station rivet-plate">
		<div class="runtime-heading">
			<span class="mono-label">{mode === 'local' ? 'Local runtime' : 'Remote API'}</span>
			<StatusPill
				state={runtimeState}
				label={app.activity === 'busy' ? 'Updating' : 'Live'}
				compact
			/>
		</div>
		<strong class="display-panel">{modelName}</strong>
		<p class="mono-micro">
			{mode === 'local' ? 'Native host' : (app.settings?.remote_endpoint ?? 'Not connected')}
		</p>
		<div class="runtime-meta">
			<span class="mono-micro">{inferencePoolSummary(poolStatus)}</span>
			<span class="mono-micro"
				>Gen {poolStatus?.generation ?? app.overview?.runtime.generation ?? '—'}</span
			>
			<span class="mono-micro">{app.settings?.sample_rate ?? 0} Hz</span>
		</div>
	</div>
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
		padding: 1.35rem 1.15rem 1.2rem;
	}

	.brand-station p {
		margin: 0.55rem 0 0;
		color: var(--ink-muted);
	}

	nav {
		display: grid;
		gap: 1px;
		padding: 0.9rem 0;
	}

	nav button {
		position: relative;
		display: grid;
		grid-template-columns: 1rem 1fr;
		align-items: center;
		gap: 0.7rem;
		width: 100%;
		min-height: 2.4rem;
		border: 0;
		padding: 0 1.15rem;
		background: transparent;
		color: var(--ink-muted);
		text-align: left;
		cursor: pointer;
		transition:
			background-color 120ms linear,
			color 120ms linear;
	}

	nav button:hover {
		background: var(--surface-2);
		color: var(--ink);
	}

	nav button.active {
		background: var(--surface-2);
		color: var(--ink);
	}

	.destination-label {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	/* Pigment, and 2px rather than 1px: this is the only rule in the window whose
	   whole job is to be found from the corner of the eye, and a 1px scarlet
	   hairline at 1.38:1 against the rail is not findable. */
	.marker {
		position: absolute;
		top: 0;
		bottom: 0;
		left: 0;
		width: 2px;
		background: var(--scarlet);
	}

	.runtime-station {
		margin: auto 0.85rem 0.85rem;
		padding: 1rem 0.95rem 1.15rem;
		/* Six fasteners across a 200px plate rather than the default four: at this
		   width four screws read as decoration that has forgotten what it is for. */
		--rivet-pitch: 16.66%;
	}

	.runtime-heading {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.6rem;
		margin: 0.35rem 0 0.7rem;
	}

	/*
		The pill is the shorter of the two, so flexbox took its width first and
		"Live" became "Li…" — a state indicator that indicates nothing. The label is
		the one allowed to wrap; the state never truncates.
	*/
	.runtime-heading .mono-label {
		min-width: 0;
	}

	.runtime-heading :global(.status) {
		flex-shrink: 0;
	}

	.runtime-station strong {
		display: block;
		color: var(--ink);
		/* Same slack reclamation as the view title: the label is scaled, so the
		   layout would otherwise reserve width the glyphs do not occupy. */
		margin-right: calc((var(--squeeze-label) - 1) * 100%);
	}

	.runtime-station p {
		margin: 0.4rem 0 0.7rem;
		overflow: hidden;
		color: var(--ink-muted);
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	/* A grid, not a wrapping flex row. Flex let the pool summary — which is as long
	   as "1 ready · 1 busy · 1 unhealthy" — push the two short readings off the
	   plate's right edge, so the fasteners ended up framing a readout that had
	   already escaped them. The summary now owns a full-width row and the two fixed
	   readings share the one under it, which is also the order you would read them
	   in: what the pool is doing, then which generation and at what rate. */
	.runtime-meta {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1px;
	}

	/* Square, hairlined cells rather than rounded pills. These are three readings
	   off one instrument, so they butt against each other on a shared 1px rule the
	   way segments of a real readout do. */
	.runtime-meta span {
		min-width: 0;
		border: 1px solid var(--line);
		padding: 0.2rem 0.4rem;
		color: var(--ink-dim);
	}

	.runtime-meta span:first-child {
		grid-column: 1 / -1;
	}

	.runtime-meta span:not(:first-child) {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	@media (max-width: 900px) {
		.brand-station {
			padding-inline: 0;
			display: grid;
			justify-items: center;
		}

		/* The wordmark goes with the labels. It was overflowing the 4.5rem rail and
		   rendering as "SHADOWOR" — a clipped brand is worse than no brand, and the
		   mark alone is already the identity at this width. */
		.brand-station :global(.display-wordmark),
		.brand-station p,
		.destination-label,
		.runtime-station strong,
		.runtime-station p,
		.runtime-meta,
		.runtime-heading .mono-label {
			display: none;
		}

		nav button {
			grid-template-columns: 1rem;
			justify-content: center;
			padding: 0;
		}

		/* The plate collapses to its glyph, and it stops being a plate: at 2.5rem
		   wide the rivets would sit on top of each other and the bevel would read as
		   a rendering artefact. Radius means "physical object", and there is no
		   longer an object here to be physical. */
		.runtime-station {
			margin: auto 0.5rem 0.85rem;
			border-radius: 0;
			border-color: var(--line);
			padding: 0.65rem 0;
			background: transparent;
			box-shadow: none;
		}

		.runtime-station::before,
		.runtime-station::after {
			display: none;
		}

		.runtime-heading {
			justify-content: center;
			margin: 0;
		}
	}
</style>

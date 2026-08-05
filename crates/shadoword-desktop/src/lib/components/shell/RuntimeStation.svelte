<script lang="ts">
	import StatusPill from '$lib/components/StatusPill.svelte';
	import { inferencePoolSummary } from '$lib/inference-pool';
	import { useDesktopShell } from '$lib/shell/desktop-shell-context';
	import type { RuntimeState } from '$lib/types';

	const shell = useDesktopShell();
	const app = shell.app;
	let mode = $derived(shell.mode);
	let poolStatus = $derived(app.overview?.status.inference_pool ?? null);
	let modelName = $derived.by(() => {
		if (mode === 'open_router') return app.settings?.openrouter_model ?? 'No model';
		const path = app.overview?.runtime.model_path;
		return app.overview?.models.find((model) => path?.endsWith(model.filename))?.name ?? 'No model';
	});
	let runtimeState = $derived<RuntimeState>(
		app.activity === 'busy'
			? 'loading'
			: mode === 'open_router'
				? app.settings?.openrouter_key_configured
					? 'ready'
					: 'offline'
				: (poolStatus?.unhealthy_units ?? 0) > 0
					? 'warning'
					: app.overview
						? 'ready'
						: 'offline'
	);
	let runtimeLabel = $derived(
		app.activity === 'busy'
			? 'Updating'
			: runtimeState === 'ready'
				? 'Live'
				: runtimeState === 'warning'
					? 'Degraded'
					: 'Offline'
	);
</script>

<div class="runtime-station rivet-plate">
	<div class="runtime-heading">
		<span class="mono-label">
			{mode === 'local'
				? 'Local runtime'
				: mode === 'open_router'
					? 'OpenRouter'
					: mode === 'remote'
						? 'Shadoword API'
						: 'Loading runtime'}
		</span>
		<StatusPill state={runtimeState} label={runtimeLabel} compact />
	</div>
	<strong class="display-panel">{modelName}</strong>
	<p class="mono-micro">
		{mode === 'local'
			? 'Native host'
			: mode === 'open_router'
				? 'openrouter.ai'
				: (app.settings?.remote_endpoint ?? 'Not connected')}
	</p>
	<div class="runtime-meta">
		<span class="mono-micro">
			{mode === 'open_router' ? 'Batch STT' : inferencePoolSummary(poolStatus)}
		</span>
		<span class="mono-micro">
			{mode === 'open_router'
				? app.settings?.openrouter_key_configured
					? 'Key set'
					: 'Key needed'
				: `Gen ${poolStatus?.generation ?? app.overview?.runtime.generation ?? '—'}`}
		</span>
		<span class="mono-micro">{app.settings?.sample_rate ?? 0} Hz</span>
	</div>
</div>

<style>
	.runtime-station {
		margin: auto 0.85rem 0.85rem;
		padding: 1rem 0.95rem 1.15rem;
		--rivet-pitch: 16.66%;
	}

	.runtime-heading {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.6rem;
		margin: 0.35rem 0 0.7rem;
	}

	.runtime-heading .mono-label {
		min-width: 0;
	}

	.runtime-heading :global(.status) {
		flex-shrink: 0;
	}

	strong {
		display: block;
		margin-right: calc((var(--squeeze-label) - 1) * 100%);
		color: var(--ink);
	}

	p {
		margin: 0.4rem 0 0.7rem;
		overflow: hidden;
		color: var(--ink-muted);
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.runtime-meta {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1px;
	}

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
		.runtime-station {
			margin: auto 0.5rem 0.85rem;
			border-radius: 0;
			border-color: var(--line);
			padding: 0.65rem 0;
			background: transparent;
			box-shadow: none;
		}

		.runtime-station::before,
		.runtime-station::after,
		strong,
		p,
		.runtime-meta,
		.runtime-heading .mono-label {
			display: none;
		}

		.runtime-heading {
			justify-content: center;
			margin: 0;
		}
	}
</style>

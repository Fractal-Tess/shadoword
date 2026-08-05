<script lang="ts">
	import { Cpu, Server } from '@lucide/svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import { Switch } from '$lib/components/ui/switch';
	import { getModelsContext } from './context';

	const context = getModelsContext();
</script>

<section class="runtime-band" aria-labelledby="runtime-title">
	<div class="runtime-identity">
		<div class="runtime-icon">
			{#if context.mode === 'remote'}<Server size={19} />{:else}<Cpu size={19} />{/if}
		</div>
		<div>
			<span>Active runtime</span>
			<h2 id="runtime-title" class="display-legend">
				{context.mode === 'remote' ? 'Shadoword API' : 'Local machine'}
			</h2>
			<p>
				{context.mode === 'remote'
					? (context.app.settings?.remote_endpoint ?? 'No endpoint configured')
					: (context.runtime?.model_path ?? 'No local model selected')}
			</p>
		</div>
	</div>
	<div class="runtime-control">
		<div>
			<span>Preload model</span>
			<small>Keep the selected model ready after startup.</small>
		</div>
		<Switch
			checked={context.preload}
			disabled={context.controlsLocked}
			onclick={() => context.updateRuntime({ preload_on_startup: !context.preload })}
			aria-label="Preload selected model"
		/>
	</div>
	<div class="runtime-health">
		<StatusPill
			state={context.app.activity === 'busy'
				? 'loading'
				: context.app.overview?.status.model_loaded
					? 'ready'
					: 'warning'}
			label={context.app.activity === 'busy'
				? 'Applying runtime'
				: context.app.overview?.status.model_loaded
					? 'Model ready'
					: 'Model unloaded'}
		/>
		<span>{context.app.overview?.status.engine ?? 'API not connected'}</span>
	</div>
</section>

<style>
	.runtime-band {
		display: grid;
		grid-template-columns: minmax(15rem, 1.3fr) minmax(13rem, 1fr) minmax(12rem, 0.8fr);
		border: 1px solid var(--line);
		background: var(--surface-1);
	}

	.runtime-band > div {
		min-width: 0;
		padding: 1rem;
	}

	.runtime-band > div + div {
		border-left: 1px solid var(--line);
	}

	.runtime-identity {
		display: grid;
		grid-template-columns: 2.6rem 1fr;
		align-items: center;
		gap: 0.8rem;
	}

	.runtime-icon {
		display: grid;
		width: 2.6rem;
		height: 2.6rem;
		place-items: center;
		border: 1px solid var(--scarlet);
		color: var(--scarlet-lamp);
	}

	.runtime-identity span,
	.runtime-control span {
		color: var(--ink-muted);
		font-size: 0.6875rem;
		font-weight: 680;
		letter-spacing: 0.09em;
		text-transform: uppercase;
	}

	.runtime-identity h2 {
		margin: 0.2rem 0 0;
		color: var(--ink);
	}

	.runtime-identity p {
		margin: 0.3rem 0 0;
		overflow: hidden;
		color: var(--ink-muted);
		font-family: var(--font-mono);
		font-size: 0.6875rem;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.runtime-control {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
	}

	.runtime-control > div {
		display: grid;
		gap: 0.25rem;
	}

	.runtime-control small,
	.runtime-health > span {
		color: var(--ink-muted);
		font-size: 0.6875rem;
		line-height: 1.4;
	}

	.runtime-health {
		display: grid;
		align-content: center;
		gap: 0.45rem;
	}

	@media (max-width: 920px) {
		.runtime-band {
			grid-template-columns: 1fr 1fr;
		}

		.runtime-health {
			display: none;
		}
	}

	@media (max-width: 720px) {
		.runtime-band {
			grid-template-columns: 1fr;
		}

		.runtime-band > div + div {
			border-top: 1px solid var(--line);
			border-left: 0;
		}
	}
</style>

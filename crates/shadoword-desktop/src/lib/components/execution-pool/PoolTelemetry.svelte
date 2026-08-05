<script lang="ts">
	import { Gauge, MemoryStick, ServerCog, Zap } from '@lucide/svelte';
	import { formatBytes } from '$lib/display';
	import { getPoolDraftContext } from '$lib/execution-pool/context.svelte';

	const state = getPoolDraftContext();
</script>

<div class="pool-telemetry" aria-live="polite">
	<div>
		<Gauge size={15} aria-hidden="true" />
		<span>Capacity</span>
		<strong
			>{state.poolStatus?.ready_units ?? 0} ready · {state.poolStatus?.busy_units ?? 0} busy · {state
				.poolStatus?.unhealthy_units ?? 0} unhealthy · {state.poolStatus?.accepting
				? 'accepting'
				: 'paused'}</strong
		>
	</div>
	<div>
		<ServerCog size={15} aria-hidden="true" />
		<span>Work</span>
		<strong
			>{state.poolStatus?.queued_jobs ?? 0} queued · {state.poolStatus?.running_jobs ?? 0} running</strong
		>
	</div>
	<div>
		<MemoryStick size={15} aria-hidden="true" />
		<span>Audio memory</span>
		<strong
			>{formatBytes(state.poolStatus?.queued_audio_bytes ?? 0)} queued · {formatBytes(
				state.poolStatus?.running_audio_bytes ?? 0
			)} running</strong
		>
	</div>
	<div>
		<Zap size={15} aria-hidden="true" />
		<span>Lifetime</span>
		<strong
			>{state.poolStatus?.completed ?? 0} complete · {state.poolStatus?.failed ?? 0} failed</strong
		>
	</div>
</div>

<style>
	.pool-telemetry {
		display: grid;
		grid-template-columns: repeat(4, minmax(0, 1fr));
		margin-top: 0.9rem;
		border-block: 1px solid var(--line);
		background: var(--surface-1);
	}

	.pool-telemetry > div {
		display: grid;
		grid-template-columns: auto 1fr;
		align-items: center;
		gap: 0.2rem 0.55rem;
		min-width: 0;
		padding: 0.75rem;
		color: var(--ink-muted);
	}

	.pool-telemetry > div + div {
		border-left: 1px solid var(--line);
	}

	.pool-telemetry :global(svg) {
		grid-row: 1 / 3;
		color: var(--ink-muted);
	}

	.pool-telemetry span {
		font-size: 0.65rem;
		font-weight: 680;
		letter-spacing: 0.07em;
		text-transform: uppercase;
	}

	.pool-telemetry strong {
		color: var(--ink-dim);
		font-family: var(--font-mono);
		font-size: 0.65rem;
		font-weight: 520;
		line-height: 1.35;
	}

	@media (max-width: 1050px) {
		.pool-telemetry {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}

		.pool-telemetry > div:nth-child(3) {
			border-left: 0;
			border-top: 1px solid var(--line);
		}

		.pool-telemetry > div:nth-child(4) {
			border-top: 1px solid var(--line);
		}
	}
</style>

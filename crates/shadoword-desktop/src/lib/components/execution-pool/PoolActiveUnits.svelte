<script lang="ts">
	import { Cpu, Zap } from '@lucide/svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import { getPoolDraftContext } from '$lib/execution-pool/context.svelte';

	const state = getPoolDraftContext();
</script>

{#if state.poolStatus?.units.length}
	<div class="active-units" aria-label="Active execution units">
		{#each state.poolStatus.units as unit (unit.id)}
			<div class="active-unit">
				<div class="unit-identity">
					{#if unit.target.kind === 'gpu'}<Zap size={16} />{:else}<Cpu size={16} />{/if}
					<div>
						<strong>{unit.id}</strong>
						<span>{state.targetLabel(unit.target)}</span>
					</div>
				</div>
				<StatusPill state={state.statusState(unit.state)} label={unit.state} compact />
				<div class="unit-counters">
					<span>{unit.required ? 'Required' : 'Optional'}</span>
					<span>{unit.completed ?? 0} complete</span>
					<span>{unit.failed ?? 0} failed</span>
				</div>
				{#if unit.last_error}<p role="status">{unit.last_error}</p>{/if}
			</div>
		{/each}
	</div>
{:else}
	<div class="empty-monitor">No execution-unit telemetry is available from this runtime.</div>
{/if}

<style>
	.active-units {
		border-bottom: 1px solid var(--line);
	}

	.active-unit {
		display: grid;
		grid-template-columns: minmax(12rem, 1fr) auto minmax(14rem, auto);
		align-items: center;
		gap: 0.8rem;
		padding: 0.72rem 0.8rem;
	}

	.active-unit + .active-unit {
		border-top: 1px solid var(--line);
	}

	.unit-identity {
		display: flex;
		align-items: center;
		gap: 0.7rem;
		min-width: 0;
		color: var(--ink-muted);
	}

	.unit-identity > div {
		display: grid;
		min-width: 0;
	}

	.unit-identity strong {
		color: var(--ink);
		font-size: 0.75rem;
	}

	.unit-identity span,
	.unit-counters,
	.active-unit p {
		color: var(--ink-muted);
		font-family: var(--font-mono);
		font-size: 0.65rem;
	}

	.unit-identity span {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.unit-counters {
		display: flex;
		justify-content: end;
		gap: 0.75rem;
	}

	.active-unit p {
		grid-column: 1 / -1;
		margin: -0.25rem 0 0 1.7rem;
		color: var(--scarlet-lamp);
	}

	.empty-monitor {
		border-bottom: 1px solid var(--line);
		padding: 1rem;
		color: var(--ink-muted);
		font-size: 0.72rem;
	}

	@media (max-width: 760px) {
		.active-unit {
			grid-template-columns: minmax(10rem, 1fr) auto;
		}

		.unit-counters {
			grid-column: 1 / -1;
			justify-content: flex-start;
			padding-left: 1.7rem;
		}
	}
</style>

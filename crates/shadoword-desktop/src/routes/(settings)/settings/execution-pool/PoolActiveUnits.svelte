<script lang="ts">
	import { Cpu, Zap } from '@lucide/svelte';
	import { StatusIndicator } from '$lib/components/ui/status-indicator';
	import { getPoolDraftContext } from './context.svelte';

	const state = getPoolDraftContext();
</script>

{#if state.poolStatus?.units.length}
	<div class="border-b border-line" aria-label="Active execution units">
		{#each state.poolStatus.units as unit (unit.id)}
			<div
				class="grid grid-cols-[minmax(12rem,1fr)_auto_minmax(14rem,auto)] items-center gap-[0.8rem] border-t border-line px-[0.8rem] py-[0.72rem] first:border-t-0 max-[760px]:grid-cols-[minmax(10rem,1fr)_auto]"
			>
				<div class="flex min-w-0 items-center gap-[0.7rem] text-ink-muted">
					{#if unit.target.kind === 'gpu'}<Zap size={16} />{:else}<Cpu size={16} />{/if}
					<div class="grid min-w-0">
						<strong class="text-xs text-ink">{unit.id}</strong>
						<span class="truncate font-mono text-[0.65rem] text-ink-muted">
							{state.targetLabel(unit.target)}
						</span>
					</div>
				</div>
				<StatusIndicator state={state.statusState(unit.state)} label={unit.state} compact />
				<div
					class="flex justify-end gap-3 font-mono text-[0.65rem] text-ink-muted max-[760px]:col-span-full max-[760px]:justify-start max-[760px]:pl-[1.7rem]"
				>
					<span>{unit.required ? 'Required' : 'Optional'}</span>
					<span>{unit.completed ?? 0} complete</span>
					<span>{unit.failed ?? 0} failed</span>
				</div>
				{#if unit.last_error}
					<p
						class="col-span-full mt-[-0.25rem] mr-0 mb-0 ml-[1.7rem] font-mono text-[0.65rem] text-scarlet-lamp"
						role="status"
					>
						{unit.last_error}
					</p>
				{/if}
			</div>
		{/each}
	</div>
{:else}
	<div class="border-b border-line p-4 text-[0.72rem] text-ink-muted">
		No execution-unit telemetry is available from this runtime.
	</div>
{/if}

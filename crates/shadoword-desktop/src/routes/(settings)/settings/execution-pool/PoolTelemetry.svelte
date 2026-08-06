<script lang="ts">
	import { Gauge, MemoryStick, ServerCog, Zap } from '@lucide/svelte';
	import { formatBytes } from '$lib/display';
	import { getPoolDraftContext } from './context.svelte';

	const state = getPoolDraftContext();
</script>

<div
	class="mt-[0.9rem] grid grid-cols-4 border-y border-line bg-plate max-[1050px]:grid-cols-2"
	aria-live="polite"
>
	<div
		class="grid min-w-0 grid-cols-[auto_1fr] items-center gap-x-[0.55rem] gap-y-[0.2rem] p-3 text-ink-muted"
	>
		<Gauge class="row-span-2 text-ink-muted" size={15} aria-hidden="true" />
		<span class="text-[0.65rem] font-[680] tracking-[0.07em] uppercase">Capacity</span>
		<strong class="font-mono text-[0.65rem] leading-[1.35] font-[520] text-ink-dim"
			>{state.poolStatus?.ready_units ?? 0} ready · {state.poolStatus?.busy_units ?? 0} busy · {state
				.poolStatus?.unhealthy_units ?? 0} unhealthy · {state.poolStatus?.accepting
				? 'accepting'
				: 'paused'}</strong
		>
	</div>
	<div
		class="grid min-w-0 grid-cols-[auto_1fr] items-center gap-x-[0.55rem] gap-y-[0.2rem] border-l border-line p-3 text-ink-muted"
	>
		<ServerCog class="row-span-2 text-ink-muted" size={15} aria-hidden="true" />
		<span class="text-[0.65rem] font-[680] tracking-[0.07em] uppercase">Work</span>
		<strong class="font-mono text-[0.65rem] leading-[1.35] font-[520] text-ink-dim"
			>{state.poolStatus?.queued_jobs ?? 0} queued · {state.poolStatus?.running_jobs ?? 0} running</strong
		>
	</div>
	<div
		class="grid min-w-0 grid-cols-[auto_1fr] items-center gap-x-[0.55rem] gap-y-[0.2rem] border-l border-line p-3 text-ink-muted max-[1050px]:border-t max-[1050px]:border-l-0"
	>
		<MemoryStick class="row-span-2 text-ink-muted" size={15} aria-hidden="true" />
		<span class="text-[0.65rem] font-[680] tracking-[0.07em] uppercase">Audio memory</span>
		<strong class="font-mono text-[0.65rem] leading-[1.35] font-[520] text-ink-dim"
			>{formatBytes(state.poolStatus?.queued_audio_bytes ?? 0)} queued · {formatBytes(
				state.poolStatus?.running_audio_bytes ?? 0
			)} running</strong
		>
	</div>
	<div
		class="grid min-w-0 grid-cols-[auto_1fr] items-center gap-x-[0.55rem] gap-y-[0.2rem] border-l border-line p-3 text-ink-muted max-[1050px]:border-t"
	>
		<Zap class="row-span-2 text-ink-muted" size={15} aria-hidden="true" />
		<span class="text-[0.65rem] font-[680] tracking-[0.07em] uppercase">Lifetime</span>
		<strong class="font-mono text-[0.65rem] leading-[1.35] font-[520] text-ink-dim"
			>{state.poolStatus?.completed ?? 0} complete · {state.poolStatus?.failed ?? 0} failed</strong
		>
	</div>
</div>

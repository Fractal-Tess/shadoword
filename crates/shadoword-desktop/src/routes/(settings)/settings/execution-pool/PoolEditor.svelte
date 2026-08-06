<script lang="ts">
	import PoolActions from './PoolActions.svelte';
	import PoolLimits from './PoolLimits.svelte';
	import PoolUnitList from './PoolUnitList.svelte';
	import { cn } from '$lib/utils';
	import { getPoolDraftContext } from './context.svelte';

	const state = getPoolDraftContext();
</script>

<div class={cn('mt-4 border border-line bg-plate', state.locked && 'opacity-[0.64]')}>
	<div class="flex items-center gap-4 border-b border-line px-4 py-[0.85rem]">
		<div>
			<h3
				class="m-0 font-display text-[1.125rem] leading-none font-normal tracking-[0.035em] text-ink uppercase"
			>
				Worker topology
			</h3>
			<p class="mt-1 mb-0 max-w-[68ch] text-[0.72rem] leading-[1.45] text-ink-muted">
				Assign CPU and GPU workers, queue capacity, and concurrency for this runtime.
			</p>
		</div>
	</div>

	<PoolUnitList />
	<PoolLimits />
	<PoolActions />
</div>

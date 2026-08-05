<script lang="ts">
	import ExecutionPool from '$lib/components/ExecutionPool.svelte';
	import { getModelsContext } from './context';

	const context = getModelsContext();
</script>

{#if context.runtime}
	{#key `${context.mode}-${context.runtime.generation ?? context.runtime.model_path}`}
		<ExecutionPool app={context.app} runtime={context.runtime} gpuDevices={context.gpuDevices} />
	{/key}
{/if}

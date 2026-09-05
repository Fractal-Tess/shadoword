<script lang="ts">
	import CustomModels from './CustomModels.svelte';
	import ModelCatalog from './ModelCatalog.svelte';
	import ModelNotice from './ModelNotice.svelte';
	import ModelsHeader from './ModelsHeader.svelte';
	import RuntimeBand from './RuntimeBand.svelte';
	import UnsupportedProvider from './UnsupportedProvider.svelte';
	import { ModelsState, setModelsContext } from './state.svelte';
	import { useDesktopShell } from '$lib/shell/desktop-shell-context';

	const state = new ModelsState(useDesktopShell().app);
	setModelsContext(state);
</script>

<svelte:head>
	<title>Models · Shadoword</title>
</svelte:head>

<div class="grid gap-4">
	<ModelsHeader />
	{#if state.mode === 'open_router'}
		<UnsupportedProvider />
	{:else}
		<RuntimeBand />
		<ModelNotice />
		<ModelCatalog />
		<CustomModels />
	{/if}
</div>

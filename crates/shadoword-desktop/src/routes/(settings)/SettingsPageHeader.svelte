<script lang="ts">
	import { SurfaceHeader } from '$lib/components/ui/surface-header';
	import { getSettingsContext } from './_state/context.svelte';
	import { settingsPageCopy, type SettingsSection } from './_state/options';
	import SettingsSaveStatus from './SettingsSaveStatus.svelte';

	let { section }: { section: SettingsSection } = $props();
	const settings = getSettingsContext();
	let pageCopy = $derived(settingsPageCopy(section, settings.mode));
</script>

<SurfaceHeader title={pageCopy.title} description={pageCopy.description}>
	{#snippet actions()}
		<SettingsSaveStatus />
	{/snippet}
</SurfaceHeader>

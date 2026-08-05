<script lang="ts">
	import { useDesktopShell } from '$lib/shell/desktop-shell-context';
	import SettingsAlerts from '$lib/components/settings/SettingsAlerts.svelte';
	import SettingsSaveStatus from '$lib/components/settings/SettingsSaveStatus.svelte';
	import SurfaceHeader from '$lib/components/SurfaceHeader.svelte';
	import { SettingsContextState, setSettingsContext } from '$lib/settings/context.svelte';
	import '$lib/settings/settings-execution.css';
	import '$lib/settings/settings-layout.css';
	import { settingsPageCopy, type SettingsSection } from '$lib/settings/options';
	import { onDestroy, untrack } from 'svelte';
	import ApplicationSettings from './settings/ApplicationSettings.svelte';
	import CaptureSettings from './settings/CaptureSettings.svelte';
	import ExecutionSettings from './settings/ExecutionSettings.svelte';
	import OutputSettings from './settings/OutputSettings.svelte';
	import TranscriptionSettings from './settings/TranscriptionSettings.svelte';

	let { section = 'settings' }: { section?: SettingsSection } = $props();
	const shell = useDesktopShell();
	const app = shell.app;
	const settings = new SettingsContextState(
		untrack(() => app),
		(page) => void shell.navigate(page)
	);
	setSettingsContext(settings);
	let pageCopy = $derived(settingsPageCopy(section, settings.mode));

	$effect(() => {
		const captureLocked = app.captureLocked;
		untrack(() => settings.persistence.handleCaptureLock(captureLocked));
	});

	onDestroy(() => settings.destroy());
</script>

<svelte:window onkeydown={(event) => settings.form.captureShortcut(event)} />

<div class="settings-view">
	<SurfaceHeader title={pageCopy.title} description={pageCopy.description}>
		{#snippet actions()}
			<SettingsSaveStatus />
		{/snippet}
	</SurfaceHeader>

	<SettingsAlerts />

	<div class="settings-sections">
		{#if section === 'settings'}
			<ExecutionSettings />
		{:else if section === 'capture'}
			<CaptureSettings />
		{:else if section === 'transcription'}
			<TranscriptionSettings />
		{:else if section === 'output'}
			<OutputSettings />
		{:else}
			<ApplicationSettings />
		{/if}
	</div>
</div>

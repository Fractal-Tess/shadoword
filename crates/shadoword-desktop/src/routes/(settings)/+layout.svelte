<script lang="ts">
	import { page } from '$app/state';
	import SettingsAlerts from './SettingsAlerts.svelte';
	import SettingsPageHeader from './SettingsPageHeader.svelte';
	import SettingsProvider from './SettingsProvider.svelte';
	import { useDesktopShell } from '$lib/shell/desktop-shell-context';
	import { settingsPageFromPathname, type SettingsPageId } from '$lib/shell/routes';

	let { children } = $props();
	const app = useDesktopShell().app;
	const titles = {
		settings: 'Execution settings',
		capture: 'Capture settings',
		transcription: 'Transcription settings',
		output: 'Output settings',
		application: 'Application settings'
	} satisfies Record<SettingsPageId, string>;
	let section = $derived(settingsPageFromPathname(page.url.pathname));
</script>

<svelte:head>
	<title>{titles[section]} · Shadoword</title>
</svelte:head>

{#if app.settings}
	{#key app.settings.mode}
		<SettingsProvider>
			<div class="grid gap-4">
				<SettingsPageHeader {section} />
				<SettingsAlerts />
				<div class="grid gap-9">
					{@render children()}
				</div>
			</div>
		</SettingsProvider>
	{/key}
{:else}
	<div
		class="border border-line p-4 font-mono text-[0.8125rem] leading-6 text-ink-muted"
		role="status"
	>
		Loading native settings…
	</div>
{/if}

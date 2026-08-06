<script lang="ts">
	import { SettingsContextState, setSettingsContext } from './_state/context.svelte';
	import { useDesktopShell } from '$lib/shell/desktop-shell-context';
	import { onDestroy, untrack, type Snippet } from 'svelte';

	let { children }: { children: Snippet } = $props();
	const shell = useDesktopShell();
	const app = shell.app;
	const settings = new SettingsContextState(
		untrack(() => app),
		(page) => void shell.navigate(page)
	);
	setSettingsContext(settings);

	const unregisterSettingsFlush = shell.registerSettingsFlush(async () => {
		settings.hideRevealedSecrets();
		await settings.persistence.flush();
	});

	$effect(() => {
		const captureLocked = app.captureLocked;
		untrack(() => settings.persistence.handleCaptureLock(captureLocked));
	});

	onDestroy(() => {
		unregisterSettingsFlush();
		settings.destroy();
	});
</script>

<svelte:window
	onkeydown={(event) => settings.form.captureShortcut(event)}
	onblur={() => settings.hideRevealedSecrets()}
/>

{@render children()}

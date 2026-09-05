<script lang="ts">
	import { RefreshCw } from '@lucide/svelte';
	import { Button } from '$lib/components/ui/button';
	import { Select } from '$lib/components/ui/select';
	import SettingsRow from '../SettingsRow.svelte';
	import { getSettingsContext } from '../_state/context.svelte';
	import { onMount } from 'svelte';
	import { MicrophoneMonitorState } from './microphone-monitor.svelte';

	const settings = getSettingsContext();
	const form = settings.form;
	const monitor = new MicrophoneMonitorState(settings.app);

	onMount(() => monitor.start());
</script>

<SettingsRow>
	<div>
		<label for="microphone" class="text-xs font-[570] text-ink">Microphone</label>
		<p class="mt-[0.2rem] text-[0.6875rem] leading-[1.45] text-ink-muted">
			Used for local, self-hosted, and OpenRouter transcription.
		</p>
	</div>
	<div
		class="flex w-[var(--control-width)] max-w-full flex-wrap items-center justify-end gap-2 max-[800px]:justify-start"
	>
		<Button
			variant="outline"
			size="sm"
			aria-label="Refresh microphone list"
			disabled={settings.locked}
			onclick={() => void settings.app.refreshInputDevices()}
		>
			<RefreshCw data-icon="inline-start" aria-hidden="true" />Refresh
		</Button>
		<Select
			id="microphone"
			value={form.microphone}
			onValueChange={(value) => form.setMicrophone(value)}
			options={settings.microphoneOptions}
			disabled={settings.locked}
			ariaLabel="Microphone"
			menuLabel="Available inputs"
		/>
	</div>
	<div
		class="col-start-2 -mt-2 grid w-[var(--control-width)] max-w-full gap-1.5 max-[800px]:col-start-1"
	>
		<div class="flex items-center justify-between gap-3 text-[0.625rem] text-ink-muted">
			<span class="font-mono tracking-[0.06em] uppercase">Selected microphone level</span>
			<span class="font-mono text-ink"
				>{monitor.monitoring ? `${monitor.percent}%` : 'Waiting'}</span
			>
		</div>
		<div
			class="h-2 overflow-hidden border border-line-strong bg-night"
			role="meter"
			aria-label="Selected microphone input level"
			aria-valuemin={0}
			aria-valuemax={100}
			aria-valuenow={monitor.percent}
			aria-valuetext={monitor.monitoring
				? `${monitor.percent}% input level`
				: 'Microphone level unavailable'}
		>
			<div
				class="h-full bg-scarlet transition-[width] duration-100 ease-out motion-reduce:transition-none"
				style:width={`${monitor.percent}%`}
			></div>
		</div>
		<p class="m-0 text-[0.625rem] leading-[1.35] text-ink-muted">
			{monitor.error ||
				(monitor.monitoring
					? 'Speak near the selected microphone to confirm its signal.'
					: 'Monitoring pauses while recording or processing.')}
		</p>
	</div>
	{#if settings.app.inputDevicesError}
		<p class="col-span-full text-[0.6875rem] text-scarlet-lamp" role="alert">
			{settings.app.inputDevicesError}
		</p>
	{/if}
</SettingsRow>

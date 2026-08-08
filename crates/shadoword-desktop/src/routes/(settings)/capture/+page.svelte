<script lang="ts">
	import { Keyboard, RefreshCw } from '@lucide/svelte';
	import { Select } from '$lib/components/ui/select';
	import { Button } from '$lib/components/ui/button';
	import SettingsPanel from '../SettingsPanel.svelte';
	import SettingsRow from '../SettingsRow.svelte';
	import SettingsSection from '../SettingsSection.svelte';
	import { getSettingsContext } from '../_state/context.svelte';
	import { PCM_FORMAT_OPTIONS, SHORTCUT_MODE_OPTIONS } from '../_state/options';
	import { commands } from '$lib/bindings';
	import { cn } from '$lib/utils';
	import { onMount } from 'svelte';

	const settings = getSettingsContext();
	const form = settings.form;
	let microphoneLevel = $state(0);
	let monitoring = $state(false);
	let meterError = $state('');
	let meterPercent = $derived(Math.round(microphoneLevel * 100));

	onMount(() => {
		let disposed = false;
		let demoPhase = 0;
		const poll = async () => {
			if (settings.app.demo) {
				demoPhase += 0.45;
				microphoneLevel = 0.12 + Math.abs(Math.sin(demoPhase)) * 0.52;
				monitoring = true;
				return;
			}
			try {
				const level = await commands.pollMicrophoneLevel();
				if (disposed) return;
				monitoring = level.monitoring;
				meterError = '';
				const peak = level.peak ?? 0;
				microphoneLevel = Math.max(peak, microphoneLevel * 0.68);
			} catch {
				if (disposed) return;
				monitoring = false;
				meterError = 'Microphone level unavailable';
				microphoneLevel = 0;
			}
		};
		void poll();
		const interval = window.setInterval(() => void poll(), 100);
		return () => {
			disposed = true;
			window.clearInterval(interval);
			if (!settings.app.demo) void commands.stopMicrophoneLevelMonitor();
		};
	});
</script>

<SettingsSection
	id="capture"
	title="Capture"
	description="Select an input and a shortcut that works anywhere on the desktop."
>
	{#snippet icon()}<Keyboard size={16} aria-hidden="true" />{/snippet}
	<SettingsPanel>
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
					<RefreshCw size={14} />Refresh
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
					<span class="font-mono text-ink">{monitoring ? `${meterPercent}%` : 'Waiting'}</span>
				</div>
				<div
					class="h-2 overflow-hidden border border-line-strong bg-night"
					role="meter"
					aria-label="Selected microphone input level"
					aria-valuemin={0}
					aria-valuemax={100}
					aria-valuenow={meterPercent}
					aria-valuetext={monitoring
						? `${meterPercent}% input level`
						: 'Microphone level unavailable'}
				>
					<div
						class="h-full bg-scarlet transition-[width] duration-100 ease-out motion-reduce:transition-none"
						style:width={`${meterPercent}%`}
					></div>
				</div>
				<p class="m-0 text-[0.625rem] leading-[1.35] text-ink-muted">
					{meterError ||
						(monitoring
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
		{#if settings.mode === 'remote' && form.transcriptionMode === 'streaming'}
			<SettingsRow>
				<div>
					<label for="streaming-pcm-format" class="text-xs font-[570] text-ink">
						Streaming PCM precision
					</label>
					<p class="mt-[0.2rem] text-[0.6875rem] leading-[1.45] text-ink-muted">
						Choose the protocol-v3 wire format used for remote live audio.
					</p>
				</div>
				<Select
					id="streaming-pcm-format"
					class="w-[var(--control-width)] max-w-full"
					value={form.streamingPcmFormat}
					onValueChange={(value) => form.setStreamingPcmFormat(value)}
					options={PCM_FORMAT_OPTIONS}
					disabled={settings.locked}
					ariaLabel="Streaming PCM precision"
				/>
			</SettingsRow>
		{/if}
		<SettingsRow>
			<div>
				<label for="shortcut-key" class="text-xs font-[570] text-ink">Global shortcut</label>
				<p class="mt-[0.2rem] text-[0.6875rem] leading-[1.45] text-ink-muted">
					Registered globally by the native desktop host when this shortcut changes.
				</p>
			</div>
			<button
				id="shortcut-key"
				class={cn(
					'h-9 w-[var(--control-width)] max-w-full cursor-pointer border border-line bg-raised px-[0.8rem] font-mono text-[0.6875rem] font-[650] text-ink outline-none focus-visible:border-ink focus-visible:ring-2 focus-visible:ring-ink/30 disabled:cursor-not-allowed disabled:opacity-[0.48]',
					form.shortcutCapturing && 'border-scarlet text-scarlet-lamp'
				)}
				type="button"
				disabled={settings.locked}
				onclick={() => form.toggleShortcutCapture()}
				aria-pressed={form.shortcutCapturing}
			>
				{form.shortcutCapturing ? 'Press shortcut…' : form.shortcut}
			</button>
		</SettingsRow>
		{#if form.shortcutError || settings.app.hotkeyError}
			<SettingsRow class="grid-cols-1 text-[0.6875rem] text-scarlet-lamp" role="alert">
				<span>{form.shortcutError || settings.app.hotkeyError}</span>
			</SettingsRow>
		{/if}
		<SettingsRow>
			<div>
				<label for="shortcut-mode" class="text-xs font-[570] text-ink">Shortcut behavior</label>
				<p class="mt-[0.2rem] text-[0.6875rem] leading-[1.45] text-ink-muted">
					Hold to speak or press once to toggle capture.
				</p>
			</div>
			<Select
				id="shortcut-mode"
				class="w-[var(--control-width)] max-w-full"
				value={form.shortcutMode}
				onValueChange={(value) => form.setShortcutMode(value)}
				options={SHORTCUT_MODE_OPTIONS}
				disabled={settings.locked}
				ariaLabel="Shortcut behavior"
			/>
		</SettingsRow>
	</SettingsPanel>
</SettingsSection>

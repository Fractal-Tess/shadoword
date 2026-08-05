<script lang="ts">
	import { Keyboard, RefreshCw } from '@lucide/svelte';
	import BrutalistSelect from '$lib/components/BrutalistSelect.svelte';
	import { Button } from '$lib/components/ui/button';
	import { getSettingsContext } from '$lib/settings/context.svelte';
	import {
		PCM_FORMAT_OPTIONS,
		SAMPLE_RATE_OPTIONS,
		SHORTCUT_MODE_OPTIONS
	} from '$lib/settings/options';

	const settings = getSettingsContext();
	const form = settings.form;
</script>

<section id="capture">
	<header>
		<div class="section-icon"><Keyboard size={16} /></div>
		<div>
			<h2 class="display-legend">Capture</h2>
			<p>Select an input and a shortcut that works anywhere on the desktop.</p>
		</div>
	</header>
	<div class="setting-list">
		<div class="setting-row">
			<div>
				<label for="microphone">Microphone</label>
				<p>Used for local, self-hosted, and OpenRouter transcription.</p>
			</div>
			<div class="inline-control">
				<Button
					variant="outline"
					size="sm"
					aria-label="Refresh microphone list"
					disabled={settings.locked}
					onclick={() => void settings.app.refreshInputDevices()}
				>
					<RefreshCw size={14} />Refresh
				</Button>
				<BrutalistSelect
					id="microphone"
					value={form.microphone}
					onValueChange={(value) => form.setMicrophone(value)}
					options={settings.microphoneOptions}
					disabled={settings.locked}
					ariaLabel="Microphone"
					menuLabel="Available inputs"
				/>
			</div>
			{#if settings.app.inputDevicesError}
				<p class="inline-error" role="alert">{settings.app.inputDevicesError}</p>
			{/if}
		</div>
		<div class="setting-row">
			<div>
				<label for="sample-rate">Capture sample rate</label>
				<p>The native recorder currently follows the selected device's default rate.</p>
			</div>
			<BrutalistSelect
				id="sample-rate"
				value={form.sampleRate}
				options={SAMPLE_RATE_OPTIONS}
				disabled
				ariaLabel="Capture sample rate"
			/>
		</div>
		{#if settings.mode === 'remote' && form.transcriptionMode === 'streaming'}
			<div class="setting-row">
				<div>
					<label for="streaming-pcm-format">Streaming PCM precision</label>
					<p>Choose the protocol-v3 wire format used for remote live audio.</p>
				</div>
				<BrutalistSelect
					id="streaming-pcm-format"
					value={form.streamingPcmFormat}
					onValueChange={(value) => form.setStreamingPcmFormat(value)}
					options={PCM_FORMAT_OPTIONS}
					disabled={settings.locked}
					ariaLabel="Streaming PCM precision"
				/>
			</div>
		{/if}
		<div class="setting-row">
			<div>
				<label for="shortcut-key">Global shortcut</label>
				<p>Registered globally by the native desktop host when this shortcut changes.</p>
			</div>
			<button
				id="shortcut-key"
				class="shortcut-key"
				class:capturing={form.shortcutCapturing}
				type="button"
				disabled={settings.locked}
				onclick={() => form.toggleShortcutCapture()}
				aria-pressed={form.shortcutCapturing}
			>
				{form.shortcutCapturing ? 'Press shortcut…' : form.shortcut}
			</button>
		</div>
		{#if form.shortcutError || settings.app.hotkeyError}
			<div class="setting-row inline-error" role="alert">
				<span>{form.shortcutError || settings.app.hotkeyError}</span>
			</div>
		{/if}
		<div class="setting-row">
			<div>
				<label for="shortcut-mode">Shortcut behavior</label>
				<p>Hold to speak or press once to toggle capture.</p>
			</div>
			<BrutalistSelect
				id="shortcut-mode"
				value={form.shortcutMode}
				onValueChange={(value) => form.setShortcutMode(value)}
				options={SHORTCUT_MODE_OPTIONS}
				disabled={settings.locked}
				ariaLabel="Shortcut behavior"
			/>
		</div>
	</div>
</section>

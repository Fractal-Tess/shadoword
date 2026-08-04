<script lang="ts">
	import {
		AlertTriangle,
		Eye,
		EyeOff,
		Keyboard,
		RefreshCw,
		ShieldCheck,
		SlidersHorizontal
	} from '@lucide/svelte';
	import type { DesktopAppState } from '$lib/app-state.svelte';
	import type {
		DesktopSettingsInput,
		HotkeyMode,
		PasteMethod,
		SecretUpdate,
		StreamingPcmFormat,
		TranscriptionMode
	} from '$lib/bindings';
	import { errorMessage } from '$lib/display';
	import { untrack } from 'svelte';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Switch } from '$lib/components/ui/switch';
	import SurfaceHeader from '$lib/components/SurfaceHeader.svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import { inferencePoolSummary, isExplicitPool } from '$lib/inference-pool';

	let { app }: { app: DesktopAppState } = $props();
	const initial = untrack(() => app.settings);
	const initialOverview = untrack(() => app.overview);
	let mode = $state(initial?.mode ?? 'remote');
	let endpoint = $state(initial?.remote_endpoint ?? 'http://127.0.0.1:47813');
	let token = $state('');
	let tokenDirty = $state(false);
	let clearToken = $state(false);
	let showToken = $state(false);
	let microphone = $state(initial?.input_device ?? '');
	let sampleRate = $state(String(initial?.sample_rate ?? 16000));
	let shortcutMode = $state<HotkeyMode>(initial?.hotkey_mode ?? 'push_to_talk');
	let shortcut = $state(initial?.hotkey_shortcut.toUpperCase() ?? 'F2');
	let shortcutCapturing = $state(false);
	let shortcutError = $state('');
	let transcriptionMode = $state<TranscriptionMode>(initial?.transcription_mode ?? 'batch');
	let streamingPcmFormat = $state<StreamingPcmFormat>(initial?.streaming_pcm_format ?? 'f32le');
	let englishOnly = $state(initialOverview?.runtime.english_only ?? initial?.english_only ?? false);
	let copyFinal = $state(initial?.copy_to_clipboard ?? true);
	let pasteMethod = $state<PasteMethod>(initial?.paste_method ?? 'none');
	let pasteDelay = $state(initial?.paste_delay_ms ?? 120);
	let closeToTray = $state(initial?.close_to_tray ?? true);
	let connectionState = $state<'idle' | 'testing' | 'success' | 'failed'>('idle');
	let saveState = $state<'saved' | 'saving' | 'failed'>('saved');
	let localError = $state('');
	let settingsLocked = $derived(app.poolMutationLocked || saveState === 'saving');
	let activeRuntime = $derived(app.overview?.runtime ?? null);
	let poolSummary = $derived(inferencePoolSummary(app.overview?.status.inference_pool));

	const testConnection = async () => {
		connectionState = 'testing';
		localError = '';
		try {
			await app.testConnection({
				endpoint,
				token: tokenDirty && !clearToken ? token : null,
				use_saved_token: !tokenDirty && !clearToken
			});
			connectionState = 'success';
		} catch (error) {
			connectionState = 'failed';
			localError = errorMessage(error);
		}
	};

	const save = async () => {
		if (!app.settings) return;
		saveState = 'saving';
		localError = '';
		const remoteToken: SecretUpdate = clearToken
			? { action: 'clear' }
			: tokenDirty
				? { action: 'set', value: token }
				: { action: 'keep' };
		const input: DesktopSettingsInput = {
			mode,
			model_path: app.settings.model_path,
			preload_on_startup: app.settings.preload_on_startup,
			whisper_accelerator: app.settings.whisper_accelerator,
			whisper_gpu_device: app.settings.whisper_gpu_device,
			remote_endpoint: endpoint,
			remote_token: remoteToken,
			input_device: microphone || null,
			sample_rate: Number(sampleRate),
			transcription_mode: transcriptionMode,
			streaming_pcm_format: streamingPcmFormat,
			english_only: englishOnly,
			copy_to_clipboard: copyFinal,
			paste_method: pasteMethod,
			paste_delay_ms: Number(pasteDelay),
			hotkey_shortcut: shortcut,
			hotkey_mode: shortcutMode,
			close_to_tray: closeToTray
		};
		try {
			await app.saveSettings(input);
			if (mode === 'remote' && app.overview) {
				await app.updateRuntime({ ...app.overview.runtime, english_only: englishOnly });
			}
			token = '';
			tokenDirty = false;
			clearToken = false;
			saveState = 'saved';
		} catch (error) {
			saveState = 'failed';
			localError = errorMessage(error);
		}
	};

	const captureShortcut = (event: KeyboardEvent) => {
		if (!shortcutCapturing || event.repeat || event.isComposing) return;
		event.preventDefault();
		event.stopPropagation();
		if (event.key === 'Escape') {
			shortcutCapturing = false;
			shortcutError = '';
			return;
		}
		if (['Control', 'Alt', 'Shift', 'Meta'].includes(event.key)) return;

		const key = shortcutKey(event.key);
		if (!key) {
			shortcutError = `Unsupported shortcut key: ${event.key}`;
			return;
		}
		const modifiers = [
			event.ctrlKey ? 'Ctrl' : null,
			event.altKey ? 'Alt' : null,
			event.shiftKey ? 'Shift' : null,
			event.metaKey ? 'Super' : null
		].filter((modifier): modifier is string => modifier !== null);
		if (key.length === 1 && modifiers.length === 0) {
			shortcutError = 'Text keys need Ctrl, Alt, Shift, or Super.';
			return;
		}

		shortcut = [...modifiers, key].join('+');
		shortcutError = '';
		shortcutCapturing = false;
	};

	function shortcutKey(key: string) {
		if (key === ' ') return 'Space';
		if (key.startsWith('Arrow')) return key.slice('Arrow'.length);
		if (/^F(?:[1-9]|1[0-9]|2[0-4])$/i.test(key)) return key.toUpperCase();
		if (key.length === 1) return key.toUpperCase();
		const supported = new Set([
			'Tab',
			'Enter',
			'Backspace',
			'Insert',
			'Delete',
			'Home',
			'End',
			'PageUp',
			'PageDown',
			'CapsLock',
			'PrintScreen',
			'ScrollLock',
			'Pause'
		]);
		return supported.has(key) ? key : null;
	}
</script>

<svelte:window onkeydown={captureShortcut} />

<div class="settings-view">
	<SurfaceHeader
		kicker="Settings"
		title="One path, tuned to you."
		description="Choose where transcription runs, how capture starts, and where completed text goes."
	>
		{#snippet actions()}
			<span class:failed={saveState === 'failed'} class="saved-state" aria-live="polite">
				{#if saveState === 'failed'}<AlertTriangle
						size={14}
					/>{:else if saveState === 'saved'}<ShieldCheck size={14} />{:else}<RefreshCw
						size={14}
					/>{/if}
				{saveState === 'failed'
					? 'Changes not saved'
					: saveState === 'saving'
						? 'Saving changes…'
						: 'Desktop configuration loaded'}
			</span>
			<Button size="sm" onclick={save} disabled={settingsLocked || !app.settings}>
				{saveState === 'saving' ? 'Saving…' : 'Save changes'}
			</Button>
		{/snippet}
	</SurfaceHeader>

	{#if saveState === 'failed'}
		<div class="save-error" role="alert">
			<AlertTriangle size={17} />
			<div>
				<strong>Shadoword could not save desktop.json</strong><span>{localError}</span>
			</div>
			<Button variant="outline" size="sm" onclick={save} disabled={settingsLocked}
				>Retry save</Button
			>
		</div>
	{/if}
	{#if app.captureLocked}
		<div class="save-error capture-lock" role="status">
			<RefreshCw size={17} />
			<div>
				<strong>Settings are locked during {app.processing ? 'finalization' : 'recording'}</strong
				><span>Stop the active session before changing native configuration.</span>
			</div>
		</div>
	{/if}

	<div class="settings-layout">
		<nav aria-label="Settings sections">
			<a href="#runtime">Runtime</a>
			<a href="#capture">Capture</a>
			<a href="#transcription">Transcription</a>
			<a href="#output">Output</a>
			<a href="#application">Application</a>
		</nav>

		<div class="settings-sections">
			<section id="runtime">
				<header>
					<div class="section-icon"><SlidersHorizontal size={16} /></div>
					<div>
						<h2 class="display-legend">Runtime</h2>
						<p>Choose where Whisper executes and how this desktop connects.</p>
					</div>
				</header>
				<div class="setting-list">
					<div class="setting-row">
						<div>
							<span class="setting-label" id="target-label">Transcription target</span>
							<p>Audio is always captured on this computer.</p>
						</div>
						<div class="segmented-control" aria-labelledby="target-label">
							<button
								class:active={mode === 'local'}
								type="button"
								disabled={settingsLocked}
								onclick={() => (mode = 'local')}
								aria-pressed={mode === 'local'}>Local</button
							>
							<button
								class:active={mode === 'remote'}
								type="button"
								disabled={settingsLocked}
								onclick={() => (mode = 'remote')}
								aria-pressed={mode === 'remote'}>Remote API</button
							>
						</div>
					</div>
					{#if mode === 'remote'}
						<div class="stacked-setting">
							<div>
								<label for="endpoint">API endpoint</label>
								<p>Use HTTPS outside an encrypted private network.</p>
							</div>
							<Input id="endpoint" bind:value={endpoint} disabled={settingsLocked} />
						</div>
						<div class="stacked-setting">
							<div>
								<label for="token">Bearer token</label>
								<p>Stored privately in the Shadoword desktop configuration.</p>
							</div>
							<div class="secret-input">
								<Input
									id="token"
									bind:value={token}
									type={showToken ? 'text' : 'password'}
									placeholder={app.settings?.remote_token_configured
										? 'Stored token unchanged'
										: 'No token configured'}
									disabled={settingsLocked}
									oninput={() => {
										tokenDirty = true;
										clearToken = false;
									}}
								/>
								<Button
									variant="ghost"
									size="icon-sm"
									onclick={() => (showToken = !showToken)}
									aria-label={showToken ? 'Hide token' : 'Show token'}
									disabled={settingsLocked}
								>
									{#if showToken}<EyeOff size={14} />{:else}<Eye size={14} />{/if}
								</Button>
							</div>
							{#if app.settings?.remote_token_configured}
								<Button
									variant="ghost"
									size="sm"
									disabled={settingsLocked}
									onclick={() => {
										clearToken = !clearToken;
										tokenDirty = false;
										token = '';
									}}
								>
									{clearToken ? 'Keep stored token' : 'Clear stored token on save'}
								</Button>
							{/if}
						</div>
						<div class="connection-row">
							<Button
								variant="outline"
								size="sm"
								onclick={testConnection}
								disabled={settingsLocked || connectionState === 'testing'}
							>
								<span class:spin={connectionState === 'testing'}><RefreshCw size={14} /></span>
								{connectionState === 'testing' ? 'Testing…' : 'Test connection'}
							</Button>
							{#if connectionState === 'success'}<StatusPill
									label={app.connectionMessage ?? 'Connected'}
								/>
							{:else if connectionState === 'failed'}<StatusPill
									state="offline"
									label="Connection failed"
								/>{/if}
						</div>
					{/if}
				</div>
			</section>

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
							<p>Used for local and remote transcription.</p>
						</div>
						<div class="inline-control">
							<Button
								variant="outline"
								size="sm"
								aria-label="Refresh microphone list"
								disabled={settingsLocked}
								onclick={() => app.refreshInputDevices()}><RefreshCw size={14} />Refresh</Button
							>
							<select id="microphone" bind:value={microphone} disabled={settingsLocked}>
								<option value="">System default</option>
								{#each app.inputDevices as device (device.name)}
									<option value={device.name}
										>{device.name}{device.is_default ? ' · default' : ''}</option
									>
								{/each}
							</select>
						</div>
						{#if app.inputDevicesError}
							<p class="inline-error" role="alert">{app.inputDevicesError}</p>
						{/if}
					</div>
					<div class="setting-row">
						<div>
							<label for="sample-rate">Capture sample rate</label>
							<p>The native recorder currently follows the selected device's default rate.</p>
						</div>
						<select id="sample-rate" bind:value={sampleRate} disabled>
							<option value="16000">16 kHz · speech</option>
							<option value="44100">44.1 kHz</option>
							<option value="48000">48 kHz · studio</option>
						</select>
					</div>
					<div class="setting-row">
						<div>
							<label for="streaming-pcm-format">Streaming PCM precision</label>
							<p>Choose the protocol-v3 wire format used for remote live audio.</p>
						</div>
						<select
							id="streaming-pcm-format"
							bind:value={streamingPcmFormat}
							disabled={settingsLocked}
						>
							<option value="s16le">16-bit integer · half bandwidth</option>
							<option value="f32le">32-bit float · capture-native</option>
						</select>
					</div>
					<div class="setting-row">
						<div>
							<label for="shortcut-key">Global shortcut</label>
							<p>Registered globally by the native desktop host when settings are saved.</p>
						</div>
						<button
							id="shortcut-key"
							class="shortcut-key"
							class:capturing={shortcutCapturing}
							type="button"
							disabled={settingsLocked}
							onclick={() => {
								shortcutCapturing = !shortcutCapturing;
								shortcutError = '';
							}}
							aria-pressed={shortcutCapturing}
						>
							{shortcutCapturing ? 'Press shortcut…' : shortcut}
						</button>
					</div>
					{#if shortcutError || app.hotkeyError}
						<div class="setting-row inline-error" role="alert">
							<span>{shortcutError || app.hotkeyError}</span>
						</div>
					{/if}
					<div class="setting-row">
						<div>
							<label for="shortcut-mode">Shortcut behavior</label>
							<p>Hold to speak or press once to toggle capture.</p>
						</div>
						<select id="shortcut-mode" bind:value={shortcutMode} disabled={settingsLocked}>
							<option value="push_to_talk">Push to talk</option><option value="toggle"
								>Toggle</option
							>
						</select>
					</div>
				</div>
			</section>

			<section id="transcription">
				<header>
					<div>
						<h2 class="display-legend">Transcription</h2>
						<p>Recognition and segmentation preferences for the active runtime.</p>
					</div>
				</header>
				<div class="setting-list">
					<div class="setting-row">
						<div>
							<label for="english-only">English recognition</label>
							<p>Constrain recognition to English instead of detecting language.</p>
						</div>
						<Switch
							id="english-only"
							bind:checked={englishOnly}
							disabled={settingsLocked}
							aria-label="English recognition"
						/>
					</div>
					<div class="setting-row">
						<div>
							<label for="streaming-segments">Stream pause-separated segments</label>
							<p>Commit pause-separated segments while recording, or transcribe once after stop.</p>
						</div>
						<Switch
							id="streaming-segments"
							checked={transcriptionMode === 'streaming'}
							disabled={settingsLocked}
							onclick={() =>
								(transcriptionMode = transcriptionMode === 'streaming' ? 'batch' : 'streaming')}
							aria-label="Stream pause-separated segments"
						/>
					</div>
				</div>
			</section>

			<section id="output">
				<header>
					<div>
						<h2 class="display-legend">Output</h2>
						<p>Control where completed transcript text is delivered.</p>
					</div>
				</header>
				<div class="setting-list">
					<div class="setting-row">
						<div>
							<label for="copy-final">Copy final transcript</label>
							<p>Write the completed transcript to the system clipboard.</p>
						</div>
						<Switch
							id="copy-final"
							bind:checked={copyFinal}
							disabled={settingsLocked}
							aria-label="Copy final transcript"
						/>
					</div>
					<div class="setting-row">
						<div>
							<label for="paste-method">Active-window delivery</label>
							<p>Type directly or paste into the active window through the native host.</p>
						</div>
						<select id="paste-method" bind:value={pasteMethod} disabled={settingsLocked}>
							<option value="none">Disabled</option><option value="direct">Type directly</option
							><option value="ctrl_v">Paste with Ctrl+V</option><option value="ctrl_shift_v"
								>Paste with Ctrl+Shift+V</option
							><option value="shift_insert">Paste with Shift+Insert</option>
						</select>
					</div>
					{#if pasteMethod !== 'none' && pasteMethod !== 'direct'}
						<div class="setting-row">
							<div>
								<label for="paste-delay">Clipboard paste delay</label>
								<p>Wait for the clipboard owner before sending the shortcut.</p>
							</div>
							<div class="delay-control">
								<Input
									id="paste-delay"
									type="number"
									min="0"
									max="1000"
									disabled={settingsLocked}
									bind:value={pasteDelay}
								/><span>ms</span>
							</div>
						</div>
						<div class="setting-row pool-summary-row">
							<div>
								<span class="setting-label">Execution topology</span>
								<p>The full pool editor and live unit telemetry are in Models.</p>
							</div>
							<div class="runtime-summary">
								<strong
									>{isExplicitPool(activeRuntime) ? 'Explicit pool' : 'Legacy single unit'}</strong
								>
								<span>{poolSummary}</span>
							</div>
						</div>
					{/if}
				</div>
			</section>

			<section id="application">
				<header>
					<div>
						<h2 class="display-legend">Application</h2>
						<p>Window and background behavior.</p>
					</div>
				</header>
				<div class="setting-list">
					<div class="setting-row">
						<div>
							<label for="close-tray">Close to tray</label>
							<p>Hide the window on close while keeping hotkeys and the tray icon active.</p>
						</div>
						<Switch
							id="close-tray"
							bind:checked={closeToTray}
							disabled={settingsLocked}
							aria-label="Close to tray"
						/>
					</div>
				</div>
			</section>
		</div>
	</div>
</div>

<style>
	.settings-view {
		display: grid;
		gap: 1rem;
	}

	.saved-state {
		display: inline-flex;
		align-items: center;
		gap: 0.45rem;
		color: var(--ink-muted);
		font-size: 0.6875rem;
	}

	.saved-state.failed {
		color: var(--scarlet-lamp);
	}

	/* The window's one error treatment, stated the same way here as in the shell: a
	   scarlet rule doubled on the leading edge over an ordinary plate. The tinted fill
	   this used to carry was scarlet mixed 95% into transparent, which on a night
	   ground is not a tint — it is nothing — so the notice was relying entirely on a
	   colour the operator could not see. */
	.save-error {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr) auto;
		align-items: center;
		gap: 0.8rem;
		border: 1px solid var(--scarlet);
		border-left-width: 2px;
		padding: 0.8rem 1rem;
		background: var(--surface-1);
		color: var(--scarlet-lamp);
	}

	.save-error > div {
		display: grid;
		gap: 0.2rem;
	}

	.save-error strong {
		color: var(--ink);
		font-size: 0.75rem;
	}

	.save-error span {
		color: var(--ink-dim);
		font-size: 0.6875rem;
	}

	/* A capture lock is not a failure — it is the app telling you to wait — so this
	   variant's whole job is to *remove* the alert styling `.save-error` sets, not to
	   swap one accent hue for another. */
	.save-error.capture-lock {
		border-color: var(--line-strong);
		background: var(--surface-1);
		color: var(--ink-dim);
	}

	.inline-error {
		color: var(--scarlet-lamp);
		font-size: 0.6875rem;
	}

	.settings-layout {
		display: grid;
		grid-template-columns: 8.5rem minmax(0, 1fr);
		align-items: start;
		gap: 1.5rem;
		border-top: 1px solid var(--line);
		padding-top: 1.25rem;
	}

	.settings-layout > nav {
		position: sticky;
		top: 0;
		display: grid;
		gap: 0.18rem;
	}

	.settings-layout > nav a {
		padding: 0.5rem 0.6rem;
		color: var(--ink-muted);
		font-size: 0.6875rem;
		font-weight: 540;
		text-decoration: none;
	}

	.settings-layout > nav a:hover,
	.settings-layout > nav a:focus-visible {
		background: var(--surface-2);
		color: var(--ink);
	}

	.settings-sections {
		display: grid;
		gap: 2.25rem;
	}

	.settings-sections section {
		scroll-margin-top: 1rem;
	}

	.settings-sections section > header {
		display: flex;
		align-items: center;
		gap: 0.7rem;
		margin-bottom: 0.75rem;
	}

	.section-icon {
		display: grid;
		width: 2rem;
		height: 2rem;
		place-items: center;
		border: 1px solid var(--line);
		color: var(--ink-dim);
	}

	h2 {
		margin: 0;
		color: var(--ink);
	}

	header p,
	.setting-row p,
	.stacked-setting p {
		margin: 0.2rem 0 0;
		color: var(--ink-muted);
		font-size: 0.6875rem;
		line-height: 1.45;
	}

	.setting-list {
		border: 1px solid var(--line);
		background: var(--surface-1);
	}

	.setting-row,
	.stacked-setting,
	.connection-row {
		padding: 0.9rem 1rem;
	}

	.setting-row {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto;
		align-items: center;
		gap: 1.5rem;
	}

	.runtime-summary {
		display: grid;
		justify-items: end;
		gap: 0.18rem;
		text-align: right;
	}

	.runtime-summary strong {
		color: var(--ink);
		font-size: 0.75rem;
		font-weight: 590;
	}

	.runtime-summary span {
		color: var(--ink-muted);
		font-family: var(--font-mono);
		font-size: 0.65rem;
	}

	.setting-list > * + * {
		border-top: 1px solid var(--line);
	}

	label,
	.setting-label {
		color: var(--ink);
		font-size: 0.75rem;
		font-weight: 570;
	}

	.stacked-setting {
		display: grid;
		grid-template-columns: minmax(12rem, 0.7fr) minmax(15rem, 1fr);
		align-items: center;
		gap: 1.5rem;
	}

	.secret-input {
		position: relative;
	}

	.secret-input :global(input) {
		padding-right: 2.5rem;
	}

	.secret-input :global(button) {
		position: absolute;
		top: 50%;
		right: 0.3rem;
		transform: translateY(-50%);
	}

	.connection-row,
	.inline-control,
	.delay-control {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.delay-control :global(input) {
		width: 6rem;
	}

	.delay-control span {
		color: var(--ink-muted);
		font-family: var(--font-mono);
		font-size: 0.6875rem;
	}

	/* The same segmented control the capture stage uses: cells butted against each
	   other on a shared 1px rule, not pills floating inside a padded trough. One
	   pattern for "pick one of these", stated identically wherever it appears. */
	.segmented-control {
		display: inline-flex;
		gap: 1px;
		background: var(--line);
	}

	.segmented-control button {
		min-width: 5.5rem;
		height: 1.95rem;
		border: 0;
		background: var(--surface-1);
		color: var(--ink-muted);
		font: inherit;
		font-size: 0.6875rem;
		cursor: pointer;
		transition:
			background-color 120ms linear,
			color 120ms linear;
	}

	.segmented-control button:hover:not(:disabled) {
		background: var(--surface-2);
		color: var(--ink);
	}

	.segmented-control button.active {
		background: var(--surface-2);
		color: var(--scarlet-lamp);
		box-shadow: inset 0 -2px 0 var(--scarlet);
	}

	select,
	.shortcut-key {
		height: 2.25rem;
		border: 1px solid var(--line);
		background: var(--surface-2);
		color: var(--ink);
		font: inherit;
		font-size: 0.6875rem;
	}

	select {
		min-width: 12rem;
		padding: 0 2rem 0 0.7rem;
	}

	.shortcut-key {
		min-width: 4.5rem;
		padding: 0 0.8rem;
		font-family: var(--font-mono);
		font-weight: 650;
		cursor: pointer;
	}

	/* Listening for a keystroke is a live state, and live is what the accent is for.
	   Hairline and lamp ink rather than a scarlet fill: the fill in this app means
	   "audio is being recorded", and a key-capture field is not that. */
	.shortcut-key.capturing {
		border-color: var(--scarlet);
		color: var(--scarlet-lamp);
	}

	.segmented-control button:disabled,
	.shortcut-key:disabled,
	select:disabled {
		cursor: not-allowed;
		opacity: 0.48;
	}

	.spin {
		display: inline-flex;
		animation: spin 1s linear infinite;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	@media (max-width: 800px) {
		.settings-layout {
			grid-template-columns: 1fr;
		}

		.settings-layout > nav {
			position: static;
			display: flex;
			overflow-x: auto;
		}

		.stacked-setting {
			grid-template-columns: 1fr;
			gap: 0.7rem;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.spin {
			animation: none;
		}
	}
</style>

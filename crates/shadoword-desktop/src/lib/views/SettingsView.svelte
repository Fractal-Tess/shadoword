<script lang="ts">
	import {
		AlertTriangle,
		AppWindow,
		ArrowRight,
		Captions,
		Cloud,
		Cpu,
		Eye,
		EyeOff,
		Keyboard,
		RadioTower,
		RefreshCw,
		Send,
		ShieldCheck,
		SlidersHorizontal
	} from '@lucide/svelte';
	import type { DesktopAppState } from '$lib/app-state.svelte';
	import type {
		DesktopSettingsInput,
		HotkeyMode,
		PasteMethod,
		SecretUpdate,
		ServiceMode,
		StreamingPcmFormat,
		TranscriptionMode
	} from '$lib/bindings';
	import { errorMessage } from '$lib/display';
	import { onDestroy, untrack } from 'svelte';
	import BrutalistSelect from '$lib/components/BrutalistSelect.svelte';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import * as Select from '$lib/components/ui/select';
	import { Switch } from '$lib/components/ui/switch';
	import SurfaceHeader from '$lib/components/SurfaceHeader.svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import { inferencePoolSummary } from '$lib/inference-pool';
	import type { PageId } from '$lib/types';

	const OPENROUTER_KEY_PATTERN = /^sk-or-v1-[a-f\d]{64}$/i;
	const SAMPLE_RATE_OPTIONS = [
		{ value: '16000', label: '16 kHz', detail: 'Speech optimized' },
		{ value: '44100', label: '44.1 kHz', detail: 'Device standard' },
		{ value: '48000', label: '48 kHz', detail: 'Studio rate' }
	];
	const PCM_FORMAT_OPTIONS = [
		{ value: 's16le', label: '16-bit integer', detail: 'Half bandwidth' },
		{ value: 'f32le', label: '32-bit float', detail: 'Capture native' }
	];
	const SHORTCUT_MODE_OPTIONS = [
		{ value: 'push_to_talk', label: 'Push to talk', detail: 'Hold while speaking' },
		{ value: 'toggle', label: 'Toggle', detail: 'Press to start and stop' }
	];
	const PASTE_METHOD_OPTIONS = [
		{ value: 'none', label: 'Disabled' },
		{ value: 'direct', label: 'Type directly' },
		{ value: 'ctrl_v', label: 'Paste with Ctrl+V' },
		{ value: 'ctrl_shift_v', label: 'Paste with Ctrl+Shift+V' },
		{ value: 'shift_insert', label: 'Paste with Shift+Insert' }
	];

	type SettingsSection = Extract<
		PageId,
		'settings' | 'capture' | 'transcription' | 'output' | 'application'
	>;

	let {
		app,
		section = 'settings',
		onNavigate = () => {}
	}: {
		app: DesktopAppState;
		section?: SettingsSection;
		onNavigate?: (page: PageId) => void;
	} = $props();
	const initial = untrack(() => app.settings);
	const initialOverview = untrack(() => app.overview);
	let mode = $state<ServiceMode>(initial?.mode ?? 'remote');
	let endpoint = $state(initial?.remote_endpoint ?? 'http://127.0.0.1:47813');
	let token = $state('');
	let tokenDirty = $state(false);
	let clearToken = $state(false);
	let showToken = $state(false);
	let openRouterModel = $state(initial?.openrouter_model ?? 'openai/whisper-large-v3');
	let openRouterKey = $state('');
	let openRouterKeyDirty = $state(false);
	let clearOpenRouterKey = $state(false);
	let showOpenRouterKey = $state(false);
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
	let openRouterConnectionState = $state<'idle' | 'testing' | 'success' | 'failed'>('idle');
	let saveState = $state<'saved' | 'pending' | 'saving' | 'failed'>('saved');
	let localError = $state('');
	let settingsLocked = $derived(app.captureLocked || saveState === 'saving');
	let activeRuntime = $derived(app.overview?.runtime ?? null);
	let poolSummary = $derived(inferencePoolSummary(app.overview?.status.inference_pool));
	let localModelName = $derived.by(() => {
		const path = app.overview?.runtime.model_path;
		return (
			app.overview?.models.find((model) => path?.endsWith(model.filename))?.name ??
			'No model selected'
		);
	});
	let selectedOpenRouterModel = $derived(
		app.openRouterModels.find((model) => model.id === openRouterModel) ?? null
	);
	let selectedOpenRouterModelName = $derived(selectedOpenRouterModel?.name ?? openRouterModel);
	let microphoneOptions = $derived([
		{ value: '', label: 'System default', detail: 'Follow the desktop audio default' },
		...app.inputDevices.map((device) => ({
			value: device.name,
			label: device.name,
			detail: device.is_default ? 'Default input' : 'Available input'
		}))
	]);
	let pageCopy = $derived(
		{
			settings: {
				kicker: 'Execution',
				title: 'Choose the execution path.',
				description: 'Run locally by default, connect to your Shadoword API, or use OpenRouter.'
			},
			capture: {
				kicker: 'Capture',
				title: 'Capture at the source.',
				description: 'Choose the microphone and global shortcut used for every execution path.'
			},
			transcription: {
				kicker: 'Transcription',
				title: 'Shape transcription.',
				description: 'Control language constraints, segmentation, and streaming precision.'
			},
			output: {
				kicker: 'Output',
				title: 'Deliver the text.',
				description: 'Choose how completed transcripts move into the active application.'
			},
			application: {
				kicker: 'Application',
				title: 'Set window behavior.',
				description: 'Control how Shadoword behaves when its window closes.'
			}
		}[section]
	);
	let autoSaveReady = false;
	let skipNextAutoSave = false;
	let keyValidationTimer: ReturnType<typeof setTimeout> | null = null;
	let saveRetryTimer: ReturnType<typeof setTimeout> | null = null;
	let saveRetryCount = 0;

	const selectMode = (next: ServiceMode) => {
		mode = next;
		if (next === 'open_router' && app.openRouterModelsState === 'idle') {
			void app.refreshOpenRouterModels();
		}
	};

	const testConnection = async () => {
		const testedEndpoint = endpoint;
		const testedToken = token;
		const testedTokenDirty = tokenDirty;
		const testedClearToken = clearToken;
		const inputIsCurrent = () =>
			endpoint === testedEndpoint &&
			token === testedToken &&
			tokenDirty === testedTokenDirty &&
			clearToken === testedClearToken;

		connectionState = 'testing';
		localError = '';
		try {
			await app.testConnection({
				endpoint: testedEndpoint,
				token: testedTokenDirty && !testedClearToken ? testedToken : null,
				use_saved_token: !testedTokenDirty && !testedClearToken
			});
			if (inputIsCurrent()) connectionState = 'success';
		} catch (error) {
			if (!inputIsCurrent()) return;
			connectionState = 'failed';
			localError = errorMessage(error);
		}
	};

	const validateOpenRouterKey = async (key: string) => {
		try {
			await app.testOpenRouterKey(key, false);
			if (openRouterKey.trim() === key) openRouterConnectionState = 'success';
		} catch {
			if (openRouterKey.trim() === key) openRouterConnectionState = 'failed';
		}
	};

	const handleOpenRouterKeyInput = () => {
		openRouterKeyDirty = true;
		clearOpenRouterKey = false;
		openRouterConnectionState = 'idle';
		app.openRouterKeyReport = null;
		if (keyValidationTimer) clearTimeout(keyValidationTimer);

		const key = openRouterKey.trim();
		if (key === '') {
			openRouterKeyDirty = false;
			return;
		}
		if (!OPENROUTER_KEY_PATTERN.test(key)) return;
		openRouterConnectionState = 'testing';
		keyValidationTimer = setTimeout(() => void validateOpenRouterKey(key), 250);
	};

	const save = async () => {
		if (!app.settings) return;
		if (tokenDirty && !clearToken && connectionState !== 'success') {
			saveState = 'pending';
			return;
		}
		if (openRouterKeyDirty && !clearOpenRouterKey && openRouterConnectionState !== 'success') {
			saveState = 'pending';
			return;
		}
		if (app.captureLocked) {
			saveState = 'pending';
			return;
		}
		saveState = 'saving';
		localError = '';
		const remoteToken: SecretUpdate = clearToken
			? { action: 'clear' }
			: tokenDirty
				? { action: 'set', value: token }
				: { action: 'keep' };
		const openRouterKeyUpdate: SecretUpdate = clearOpenRouterKey
			? { action: 'clear' }
			: openRouterKeyDirty
				? { action: 'set', value: openRouterKey }
				: { action: 'keep' };
		const input: DesktopSettingsInput = {
			mode,
			model_path: app.settings.model_path,
			preload_on_startup: app.settings.preload_on_startup,
			whisper_accelerator: app.settings.whisper_accelerator,
			whisper_gpu_device: app.settings.whisper_gpu_device,
			remote_endpoint: endpoint,
			remote_token: remoteToken,
			openrouter_model: openRouterModel,
			openrouter_key: openRouterKeyUpdate,
			input_device: microphone || null,
			sample_rate: Number(sampleRate),
			transcription_mode: mode === 'open_router' ? 'batch' : transcriptionMode,
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
			skipNextAutoSave =
				token !== '' ||
				tokenDirty ||
				clearToken ||
				openRouterKey !== '' ||
				openRouterKeyDirty ||
				clearOpenRouterKey;
			token = '';
			tokenDirty = false;
			clearToken = false;
			openRouterKey = '';
			openRouterKeyDirty = false;
			clearOpenRouterKey = false;
			saveRetryCount = 0;
			saveState = 'saved';
		} catch (error) {
			localError = errorMessage(error);
			if (saveRetryCount < 2) {
				saveRetryCount += 1;
				saveState = 'pending';
				saveRetryTimer = setTimeout(() => {
					saveRetryTimer = null;
					void save();
				}, 900 * saveRetryCount);
			} else {
				saveState = 'failed';
			}
		}
	};

	$effect(() => {
		const formState = [
			mode,
			endpoint,
			connectionState,
			token,
			tokenDirty,
			clearToken,
			openRouterModel,
			openRouterKey,
			openRouterKeyDirty,
			clearOpenRouterKey,
			openRouterConnectionState,
			microphone,
			sampleRate,
			shortcutMode,
			shortcut,
			transcriptionMode,
			streamingPcmFormat,
			englishOnly,
			copyFinal,
			pasteMethod,
			pasteDelay,
			closeToTray
		];
		void formState;

		if (!autoSaveReady) {
			autoSaveReady = true;
			return;
		}
		if (skipNextAutoSave) {
			skipNextAutoSave = false;
			return;
		}
		if (!untrack(() => app.settings)) return;
		if (saveRetryTimer) {
			clearTimeout(saveRetryTimer);
			saveRetryTimer = null;
			saveRetryCount = 0;
		}
		if (tokenDirty && !clearToken && connectionState !== 'success') {
			saveState = 'pending';
			return;
		}
		if (openRouterKeyDirty && !clearOpenRouterKey && openRouterConnectionState !== 'success') {
			saveState = 'pending';
			return;
		}
		const parsedPasteDelay = Number(pasteDelay);
		if (!Number.isInteger(parsedPasteDelay) || parsedPasteDelay < 0 || parsedPasteDelay > 1000) {
			saveState = 'failed';
			localError = 'Paste delay must be a whole number from 0 to 1000 milliseconds.';
			return;
		}

		if (app.captureLocked) {
			saveState = 'pending';
			return;
		}

		saveState = 'pending';
		const timeout = window.setTimeout(() => void save(), 650);
		return () => window.clearTimeout(timeout);
	});

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

	onDestroy(() => {
		if (saveState === 'pending') void save();
		if (keyValidationTimer) clearTimeout(keyValidationTimer);
		if (saveRetryTimer) clearTimeout(saveRetryTimer);
	});

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
	<SurfaceHeader kicker={pageCopy.kicker} title={pageCopy.title} description={pageCopy.description}>
		{#snippet actions()}
			<span class:failed={saveState === 'failed'} class="saved-state" aria-live="polite">
				{#if saveState === 'failed'}
					<AlertTriangle size={14} />
				{:else if saveState === 'saved'}
					<ShieldCheck size={14} />
				{:else}
					<span class:spin={saveState === 'saving'}><RefreshCw size={14} /></span>
				{/if}
				{saveState === 'failed'
					? 'Changes not saved'
					: saveState === 'pending'
						? 'Waiting to save…'
						: saveState === 'saving'
							? 'Saving changes…'
							: 'All changes saved'}
			</span>
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

	<div class="settings-sections">
		{#if section === 'settings'}
			<section id="runtime">
				<header>
					<div class="section-icon"><SlidersHorizontal size={16} /></div>
					<div>
						<h2 class="display-legend">Runtime</h2>
						<p>Choose where transcription runs and how this desktop connects.</p>
					</div>
				</header>
				<div class="setting-list">
					<div class="target-grid" aria-label="Execution target">
						<button
							class:active={mode === 'local'}
							type="button"
							disabled={settingsLocked || !app.settings?.local_runtime_available}
							onclick={() => selectMode('local')}
							aria-pressed={mode === 'local'}
						>
							<span class="target-icon"><Cpu size={20} /></span>
							<span class="target-copy">
								<strong>Local execution</strong>
								<small>Whisper runs on this machine</small>
							</span>
							<span class="target-state">
								{app.settings?.local_runtime_available
									? 'Default · private'
									: 'Requires full desktop build'}
							</span>
						</button>
						<button
							class:active={mode === 'remote'}
							type="button"
							disabled={settingsLocked}
							onclick={() => selectMode('remote')}
							aria-pressed={mode === 'remote'}
						>
							<span class="target-icon"><RadioTower size={20} /></span>
							<span class="target-copy">
								<strong>Shadoword API</strong>
								<small>Your self-hosted inference host</small>
							</span>
							<span class="target-state">Private network</span>
						</button>
						<button
							class:active={mode === 'open_router'}
							type="button"
							disabled={settingsLocked}
							onclick={() => selectMode('open_router')}
							aria-pressed={mode === 'open_router'}
						>
							<span class="target-icon"><Cloud size={20} /></span>
							<span class="target-copy">
								<strong>OpenRouter</strong>
								<small>Direct managed transcription</small>
							</span>
							<span class="target-state">Cloud · batch</span>
						</button>
					</div>
					{#if mode === 'local'}
						<div class="setting-row local-runtime-row">
							<div>
								<span class="setting-label">Active model</span>
								<p>Model weights stay on this machine.</p>
							</div>
							<div class="runtime-summary">
								<strong>{localModelName}</strong>
								<span>{activeRuntime?.whisper_accelerator ?? 'CPU'} · {poolSummary}</span>
							</div>
						</div>
						<div class="setting-row">
							<div>
								<span class="setting-label">Models and execution pool</span>
								<p>Manage model downloads, accelerator affinity, and worker topology.</p>
							</div>
							<Button variant="outline" size="sm" onclick={() => onNavigate('models')}>
								Open runtime <ArrowRight size={14} />
							</Button>
						</div>
					{/if}
					{#if mode === 'remote'}
						<div class="stacked-setting">
							<div>
								<label for="endpoint">API endpoint</label>
								<p>Use HTTPS outside an encrypted private network.</p>
							</div>
							<Input
								id="endpoint"
								bind:value={endpoint}
								disabled={settingsLocked}
								oninput={() => (connectionState = 'idle')}
							/>
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
										tokenDirty = token.trim() !== '';
										clearToken = false;
										connectionState = 'idle';
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
									{clearToken ? 'Keep stored token' : 'Clear stored token'}
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
								/>
							{:else if tokenDirty}<span class="verification-note"
									>Test to verify and save this token.</span
								>{/if}
						</div>
					{/if}
					{#if mode === 'open_router'}
						<div class="stacked-setting">
							<div>
								<label for="openrouter-model">Transcription model</label>
								<p>Use an OpenRouter model with transcription output.</p>
							</div>
							<div class="model-picker">
								<Select.Root
									type="single"
									bind:value={openRouterModel}
									disabled={settingsLocked || app.openRouterModelsState === 'loading'}
								>
									<Select.Trigger id="openrouter-model" class="model-select-trigger">
										<span class="model-select-value">
											<strong>{selectedOpenRouterModelName}</strong>
											<code>{openRouterModel}</code>
										</span>
									</Select.Trigger>
									<Select.Content class="model-select-content" sideOffset={6}>
										<Select.Label>
											{app.openRouterModels.length} transcription models
										</Select.Label>
										{#if !app.openRouterModels.some((model) => model.id === openRouterModel)}
											<Select.Item
												value={openRouterModel}
												label={openRouterModel}
												class="model-select-item"
											>
												<span class="model-option-copy">
													<strong>Current model</strong>
													<code>{openRouterModel}</code>
												</span>
											</Select.Item>
										{/if}
										{#each app.openRouterModels as model (model.id)}
											<Select.Item value={model.id} label={model.name} class="model-select-item">
												<span class="model-option-copy">
													<strong>{model.name}</strong>
													<code>{model.id}</code>
												</span>
											</Select.Item>
										{/each}
									</Select.Content>
								</Select.Root>
								<Button
									variant="outline"
									size="sm"
									aria-label="Refresh OpenRouter transcription models"
									onclick={() => app.refreshOpenRouterModels()}
									disabled={settingsLocked || app.openRouterModelsState === 'loading'}
								>
									<span class:spin={app.openRouterModelsState === 'loading'}>
										<RefreshCw size={14} />
									</span>
									{app.openRouterModelsState === 'loading' ? 'Syncing…' : 'Sync models'}
								</Button>
							</div>
							{#if selectedOpenRouterModel}
								<p class="model-description">{selectedOpenRouterModel.description}</p>
							{:else if app.openRouterModelsError}
								<p class="inline-error" role="alert">{app.openRouterModelsError}</p>
							{/if}
						</div>
						<div class="stacked-setting">
							<div>
								<label for="openrouter-key">OpenRouter API key</label>
								<p>Stored only in the native Shadoword desktop configuration.</p>
							</div>
							<div class="secret-input">
								<Input
									id="openrouter-key"
									bind:value={openRouterKey}
									type={showOpenRouterKey ? 'text' : 'password'}
									placeholder={app.settings?.openrouter_key_configured
										? 'Stored key unchanged'
										: 'Enter an OpenRouter API key'}
									disabled={settingsLocked}
									oninput={handleOpenRouterKeyInput}
								/>
								<Button
									variant="ghost"
									size="icon-sm"
									onclick={() => (showOpenRouterKey = !showOpenRouterKey)}
									aria-label={showOpenRouterKey ? 'Hide OpenRouter key' : 'Show OpenRouter key'}
									disabled={settingsLocked}
								>
									{#if showOpenRouterKey}<EyeOff size={14} />{:else}<Eye size={14} />{/if}
								</Button>
							</div>
							{#if app.settings?.openrouter_key_configured}
								<Button
									variant="ghost"
									size="sm"
									disabled={settingsLocked}
									onclick={() => {
										clearOpenRouterKey = !clearOpenRouterKey;
										openRouterKeyDirty = false;
										openRouterKey = '';
										openRouterConnectionState = 'idle';
										app.openRouterKeyReport = null;
									}}
								>
									{clearOpenRouterKey ? 'Keep stored key' : 'Clear stored key'}
								</Button>
							{/if}
							<div
								class:valid={openRouterConnectionState === 'success'}
								class="key-validation"
								aria-live="polite"
							>
								{#if openRouterConnectionState === 'testing'}
									<span class="spin"><RefreshCw size={14} /></span>
									<span>Checking key with OpenRouter…</span>
								{:else if openRouterConnectionState === 'success'}
									<ShieldCheck size={15} />
									<strong>API key verified</strong>
								{:else if openRouterConnectionState === 'failed'}
									<AlertTriangle size={15} />
									<span>OpenRouter rejected this key. Check it and try again.</span>
								{:else if openRouterKeyDirty && openRouterKey.trim() !== ''}
									<span
										>{openRouterKey.trim().length} / 73 characters · validation starts when complete</span
									>
								{:else if app.settings?.openrouter_key_configured}
									<ShieldCheck size={15} />
									<span>Stored API key</span>
								{:else}
									<span>Keys are validated automatically and saved only after verification.</span>
								{/if}
							</div>
							{#if openRouterConnectionState === 'success' && app.openRouterKeyReport}
								<p class="key-report">
									{app.openRouterKeyReport.label ?? 'OpenRouter key'} ·
									{app.openRouterKeyReport.limit_remaining == null
										? 'No credit limit reported'
										: `${app.openRouterKeyReport.limit_remaining.toFixed(4)} credits remaining`}
								</p>
							{/if}
						</div>
						<p class="provider-note">
							Audio is sent to OpenRouter only after recording stops. OpenRouter mode uses batch
							transcription.
						</p>
					{/if}
				</div>
			</section>
		{/if}

		{#if section === 'capture'}
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
								disabled={settingsLocked}
								onclick={() => app.refreshInputDevices()}><RefreshCw size={14} />Refresh</Button
							>
							<BrutalistSelect
								id="microphone"
								bind:value={microphone}
								options={microphoneOptions}
								disabled={settingsLocked}
								ariaLabel="Microphone"
								menuLabel="Available inputs"
							/>
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
						<BrutalistSelect
							id="sample-rate"
							bind:value={sampleRate}
							options={SAMPLE_RATE_OPTIONS}
							disabled
							ariaLabel="Capture sample rate"
						/>
					</div>
					<div class="setting-row">
						<div>
							<label for="streaming-pcm-format">Streaming PCM precision</label>
							<p>Choose the protocol-v3 wire format used for remote live audio.</p>
						</div>
						<BrutalistSelect
							id="streaming-pcm-format"
							bind:value={streamingPcmFormat}
							options={PCM_FORMAT_OPTIONS}
							disabled={settingsLocked || mode === 'open_router'}
							ariaLabel="Streaming PCM precision"
						/>
					</div>
					<div class="setting-row">
						<div>
							<label for="shortcut-key">Global shortcut</label>
							<p>Registered globally by the native desktop host when this shortcut changes.</p>
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
						<BrutalistSelect
							id="shortcut-mode"
							bind:value={shortcutMode}
							options={SHORTCUT_MODE_OPTIONS}
							disabled={settingsLocked}
							ariaLabel="Shortcut behavior"
						/>
					</div>
				</div>
			</section>
		{/if}

		{#if section === 'transcription'}
			<section id="transcription">
				<header>
					<div class="section-icon"><Captions size={16} /></div>
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
							<p>
								{mode === 'open_router'
									? 'OpenRouter receives one WAV file after recording stops.'
									: 'Commit pause-separated segments while recording, or transcribe once after stop.'}
							</p>
						</div>
						<Switch
							id="streaming-segments"
							checked={mode !== 'open_router' && transcriptionMode === 'streaming'}
							disabled={settingsLocked || mode === 'open_router'}
							onclick={() =>
								(transcriptionMode = transcriptionMode === 'streaming' ? 'batch' : 'streaming')}
							aria-label="Stream pause-separated segments"
						/>
					</div>
				</div>
			</section>
		{/if}

		{#if section === 'output'}
			<section id="output">
				<header>
					<div class="section-icon"><Send size={16} /></div>
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
						<BrutalistSelect
							id="paste-method"
							bind:value={pasteMethod}
							options={PASTE_METHOD_OPTIONS}
							disabled={settingsLocked}
							ariaLabel="Active-window delivery"
						/>
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
					{/if}
				</div>
			</section>
		{/if}

		{#if section === 'application'}
			<section id="application">
				<header>
					<div class="section-icon"><AppWindow size={16} /></div>
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
		{/if}
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

	.provider-note {
		margin: 0;
		border-left: 2px solid var(--scarlet);
		padding: 0.65rem 0.8rem;
		background: var(--surface-1);
		color: var(--ink-muted);
		font-size: 0.6875rem;
		line-height: 1.55;
	}

	.settings-sections {
		display: grid;
		gap: 2.25rem;
		border-top: 1px solid var(--line);
		padding-top: 1.25rem;
	}

	.target-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(13rem, 1fr));
		gap: 1px;
		padding: 1px;
		background: var(--line);
	}

	.target-grid button {
		position: relative;
		display: grid;
		grid-template-columns: auto minmax(0, 1fr);
		grid-template-rows: 1fr auto;
		align-items: start;
		gap: 0.8rem;
		min-height: 8.5rem;
		border: 0;
		padding: 1rem;
		background: var(--surface-1);
		color: var(--ink-muted);
		font: inherit;
		text-align: left;
		cursor: pointer;
		transition:
			background-color 120ms linear,
			color 120ms linear;
	}

	.target-grid button:hover:not(:disabled),
	.target-grid button:focus-visible {
		background: var(--surface-2);
		color: var(--ink);
	}

	.target-grid button:focus-visible {
		outline: 2px solid var(--ink);
		outline-offset: -2px;
	}

	.target-grid button.active {
		box-shadow: inset 0 2px 0 var(--scarlet);
		background: var(--surface-2);
	}

	.target-grid button:disabled {
		cursor: not-allowed;
		opacity: 0.55;
	}

	.target-icon {
		display: grid;
		width: 2.25rem;
		height: 2.25rem;
		place-items: center;
		border: 1px solid var(--line-strong);
		color: var(--ink-dim);
	}

	.target-grid button.active .target-icon {
		border-color: var(--scarlet);
		color: var(--scarlet-lamp);
	}

	.target-copy {
		display: grid;
		gap: 0.25rem;
	}

	.target-copy strong {
		color: var(--ink);
		font-size: 0.76rem;
		font-weight: 620;
	}

	.target-copy small,
	.target-state {
		color: var(--ink-muted);
		font-size: 0.6rem;
		line-height: 1.4;
	}

	.target-state {
		grid-column: 1 / -1;
		align-self: end;
		letter-spacing: 0.04em;
		text-transform: uppercase;
	}

	.target-grid button.active .target-state {
		color: var(--scarlet-lamp);
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
	.model-picker,
	.delay-control,
	.key-validation {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.key-validation {
		grid-column: 2;
		min-height: 1.8rem;
		padding: 0;
		color: var(--ink-muted);
		font-size: 0.64rem;
	}

	.key-validation strong {
		color: var(--ink);
		font-size: inherit;
	}

	.verification-note {
		color: var(--ink-muted);
		font-size: 0.64rem;
	}

	.key-validation.valid,
	.key-validation.valid strong {
		color: #5ee28a;
	}

	.key-validation:has(> :global(.lucide-triangle-alert)) {
		color: var(--scarlet-lamp);
	}

	.model-picker > :global(*) {
		min-width: 0;
	}

	.model-picker > :global([data-slot='select-trigger']) {
		flex: 1;
	}

	:global(.model-select-trigger) {
		height: 3rem;
		padding: 0.45rem 0.7rem;
		border-color: var(--line-strong);
		background: var(--surface-2);
		text-align: left;
	}

	:global(.model-select-trigger:hover:not(:disabled)) {
		border-color: var(--ink-muted);
		background: color-mix(in srgb, var(--surface-2) 88%, var(--ink) 12%);
	}

	.model-select-value,
	.model-option-copy {
		display: grid;
		min-width: 0;
		gap: 0.12rem;
	}

	.model-select-value {
		flex: 1;
	}

	.model-select-value strong,
	.model-option-copy strong {
		overflow: hidden;
		color: var(--ink);
		font-size: 0.7rem;
		font-weight: 590;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.model-select-value code,
	.model-option-copy code {
		overflow: hidden;
		color: var(--ink-muted);
		font-family: var(--font-mono);
		font-size: 0.58rem;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	:global(.model-select-content) {
		width: min(31rem, var(--bits-select-anchor-width));
		max-height: 22rem;
		border: 1px solid var(--line-strong);
		background: var(--surface-1);
		box-shadow: 0 12px 32px rgb(0 0 0 / 42%);
	}

	:global(.model-select-content [data-slot='select-label']) {
		padding: 0.65rem 0.75rem 0.5rem;
		border-bottom: 1px solid var(--line);
		color: var(--ink-muted);
		font-size: 0.58rem;
		letter-spacing: 0.08em;
		text-transform: uppercase;
	}

	:global(.model-select-item) {
		min-height: 3rem;
		padding: 0.55rem 2.25rem 0.55rem 0.75rem;
		border-bottom: 1px solid var(--line);
	}

	:global(.model-select-item:last-child) {
		border-bottom: 0;
	}

	:global(.model-select-item[data-highlighted]) {
		background: var(--surface-2);
	}

	:global(.model-select-item[data-selected]) {
		box-shadow: inset 2px 0 0 var(--scarlet);
	}

	.model-description,
	.key-report,
	.stacked-setting > .inline-error {
		grid-column: 2;
		color: var(--ink-muted);
		font-size: 0.65rem;
		line-height: 1.5;
	}

	.model-description {
		margin-top: -0.8rem;
	}

	.key-report {
		margin: -0.55rem 0 0;
	}

	.delay-control :global(input) {
		width: 6rem;
	}

	.delay-control span {
		color: var(--ink-muted);
		font-family: var(--font-mono);
		font-size: 0.6875rem;
	}

	.shortcut-key {
		height: 2.25rem;
		border: 1px solid var(--line);
		background: var(--surface-2);
		color: var(--ink);
		font: inherit;
		font-size: 0.6875rem;
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

	.shortcut-key:disabled {
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

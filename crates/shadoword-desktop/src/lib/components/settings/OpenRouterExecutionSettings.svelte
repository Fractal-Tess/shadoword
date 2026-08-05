<script lang="ts">
	import { AlertTriangle, Eye, EyeOff, RefreshCw, ShieldCheck } from '@lucide/svelte';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import * as Select from '$lib/components/ui/select';
	import { getSettingsContext } from '$lib/settings/context.svelte';
	import { onMount } from 'svelte';

	const settings = getSettingsContext();
	const openRouter = settings.openRouter;
	let selectedModel = $derived(
		settings.app.openRouterModels.find((model) => model.id === openRouter.model) ?? null
	);
	let selectedModelName = $derived(selectedModel?.name ?? openRouter.model);

	onMount(() => {
		if (settings.app.openRouterModelsState === 'idle') {
			void settings.app.refreshOpenRouterModels();
		}
	});
</script>

<div class="stacked-setting">
	<div>
		<label for="openrouter-model">Transcription model</label>
		<p>Use an OpenRouter model with transcription output.</p>
	</div>
	<div class="model-picker">
		<Select.Root
			type="single"
			value={openRouter.model}
			onValueChange={(value) => {
				if (value) openRouter.setModel(value);
			}}
			disabled={settings.locked || settings.app.openRouterModelsState === 'loading'}
		>
			<Select.Trigger id="openrouter-model" class="model-select-trigger">
				<span class="model-select-value">
					<strong>{selectedModelName}</strong>
					<code>{openRouter.model}</code>
				</span>
			</Select.Trigger>
			<Select.Content class="model-select-content" sideOffset={6}>
				<Select.Label>
					{settings.app.openRouterModels.length} transcription models
				</Select.Label>
				{#if !settings.app.openRouterModels.some((model) => model.id === openRouter.model)}
					<Select.Item value={openRouter.model} label={openRouter.model} class="model-select-item">
						<span class="model-option-copy">
							<strong>Current model</strong>
							<code>{openRouter.model}</code>
						</span>
					</Select.Item>
				{/if}
				{#each settings.app.openRouterModels as model (model.id)}
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
			onclick={() => void settings.app.refreshOpenRouterModels()}
			disabled={settings.locked || settings.app.openRouterModelsState === 'loading'}
		>
			<span class:spin={settings.app.openRouterModelsState === 'loading'}>
				<RefreshCw size={14} />
			</span>
			{settings.app.openRouterModelsState === 'loading' ? 'Syncing…' : 'Sync models'}
		</Button>
	</div>
	{#if selectedModel}
		<p class="model-description">{selectedModel.description}</p>
	{:else if settings.app.openRouterModelsError}
		<p class="inline-error" role="alert">{settings.app.openRouterModelsError}</p>
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
			value={openRouter.key}
			type={openRouter.showKey ? 'text' : 'password'}
			placeholder={settings.app.settings?.openrouter_key_configured
				? 'Stored key unchanged'
				: 'Enter an OpenRouter API key'}
			disabled={settings.locked}
			oninput={(event) => openRouter.setKey(event.currentTarget.value)}
		/>
		<Button
			variant="ghost"
			size="icon-sm"
			onclick={() => openRouter.toggleKeyVisibility()}
			aria-label={openRouter.showKey ? 'Hide OpenRouter key' : 'Show OpenRouter key'}
			disabled={settings.locked}
		>
			{#if openRouter.showKey}<EyeOff size={14} />{:else}<Eye size={14} />{/if}
		</Button>
	</div>
	{#if settings.app.settings?.openrouter_key_configured}
		<Button
			variant="ghost"
			size="sm"
			disabled={settings.locked}
			onclick={() => openRouter.toggleClearKey()}
		>
			{openRouter.clearKey ? 'Keep stored key' : 'Clear stored key'}
		</Button>
	{/if}
	<div
		class:valid={openRouter.connectionState === 'success'}
		class="key-validation"
		aria-live="polite"
	>
		{#if openRouter.connectionState === 'testing'}
			<span class="spin"><RefreshCw size={14} /></span>
			<span>Checking key with OpenRouter…</span>
		{:else if openRouter.connectionState === 'success'}
			<ShieldCheck size={15} />
			<strong>API key verified</strong>
		{:else if openRouter.connectionState === 'failed'}
			<AlertTriangle size={15} />
			<span>OpenRouter rejected this key. Check it and try again.</span>
		{:else if openRouter.keyDirty && openRouter.key.trim() !== ''}
			<span>{openRouter.key.trim().length} / 73 characters · validation starts when complete</span>
		{:else if settings.app.settings?.openrouter_key_configured}
			<ShieldCheck size={15} />
			<span>Stored API key</span>
		{:else}
			<span>Keys are validated automatically and saved only after verification.</span>
		{/if}
	</div>
	{#if openRouter.connectionState === 'success' && openRouter.keyReport}
		<p class="key-report">
			{openRouter.keyReport.label ?? 'OpenRouter key'} ·
			{openRouter.keyReport.limit_remaining == null
				? 'No credit limit reported'
				: `${openRouter.keyReport.limit_remaining.toFixed(4)} credits remaining`}
		</p>
	{/if}
</div>

<p class="provider-note">
	Audio is sent to OpenRouter only after recording stops. OpenRouter mode uses batch transcription.
</p>

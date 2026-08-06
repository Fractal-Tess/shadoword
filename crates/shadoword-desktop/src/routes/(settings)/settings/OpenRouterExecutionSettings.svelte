<script lang="ts">
	import { AlertTriangle, Copy, Eye, EyeOff, RefreshCw, ShieldCheck } from '@lucide/svelte';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Select } from '$lib/components/ui/select';
	import SettingsRow from '../SettingsRow.svelte';
	import { getSettingsContext } from '../_state/context.svelte';
	import { cn } from '$lib/utils';
	import { onMount } from 'svelte';

	const settings = getSettingsContext();
	const openRouter = settings.openRouter;
	const rowClass = 'grid-cols-[minmax(12rem,0.7fr)_minmax(15rem,1fr)] max-[800px]:grid-cols-1';
	let selectedModel = $derived(
		settings.app.openRouterModels.find((model) => model.id === openRouter.model) ?? null
	);
	let modelOptions = $derived.by(() => {
		const options = settings.app.openRouterModels.map((model) => ({
			value: model.id,
			label: model.name,
			detail: model.id
		}));
		if (selectedModel || !openRouter.model) return options;
		return [
			{ value: openRouter.model, label: 'Current model', detail: openRouter.model },
			...options
		];
	});

	onMount(() => {
		if (settings.app.openRouterModelsState === 'idle') {
			void settings.app.refreshOpenRouterModels();
		}
	});
</script>

<SettingsRow class={rowClass}>
	<div>
		<label for="openrouter-model" class="text-xs font-[570] text-ink">Transcription model</label>
		<p class="mt-[0.2rem] text-[0.6875rem] leading-[1.45] text-ink-muted">
			Use an OpenRouter model with transcription output.
		</p>
	</div>
	<div class="flex min-w-0 flex-wrap items-center gap-2">
		<Select
			id="openrouter-model"
			class="h-12 min-w-0 flex-1 border-line-strong bg-raised px-[0.7rem] py-[0.45rem] text-left hover:border-ink-muted hover:bg-[color-mix(in_srgb,var(--surface-2)_88%,var(--ink)_12%)]"
			contentClass="max-h-[22rem] w-[min(31rem,var(--bits-select-anchor-width))]"
			itemClass="min-h-12 px-3 py-[0.55rem] pr-9"
			value={openRouter.model}
			onValueChange={(value) => openRouter.setModel(value)}
			options={modelOptions}
			disabled={settings.locked || settings.app.openRouterModelsState === 'loading'}
			ariaLabel="OpenRouter transcription model"
			ariaBusy={settings.app.openRouterModelsState === 'loading'}
			menuLabel={`${settings.app.openRouterModels.length} transcription models`}
		/>
		<Button
			variant="outline"
			size="sm"
			aria-label="Refresh OpenRouter transcription models"
			onclick={() => void settings.app.refreshOpenRouterModels()}
			disabled={settings.locked || settings.app.openRouterModelsState === 'loading'}
		>
			<span
				class={cn(
					'inline-flex',
					settings.app.openRouterModelsState === 'loading' &&
						'animate-spin motion-reduce:animate-none'
				)}
			>
				<RefreshCw size={14} />
			</span>
			{settings.app.openRouterModelsState === 'loading' ? 'Syncing…' : 'Sync models'}
		</Button>
	</div>
	{#if selectedModel}
		<p
			class="col-start-2 -mt-[0.8rem] text-[0.6875rem] leading-normal text-ink-muted max-[800px]:col-start-1"
		>
			{selectedModel.description}
		</p>
	{:else if settings.app.openRouterModelsError}
		<p class="col-start-2 text-[0.6875rem] text-scarlet-lamp max-[800px]:col-start-1" role="alert">
			{settings.app.openRouterModelsError}
		</p>
	{/if}
</SettingsRow>

<SettingsRow class={rowClass}>
	<div>
		<label for="openrouter-key" class="text-xs font-[570] text-ink">OpenRouter API key</label>
		<p class="mt-[0.2rem] text-[0.6875rem] leading-[1.45] text-ink-muted">
			Stored only in the native Shadoword desktop configuration after verification.
		</p>
	</div>
	<div class="flex min-w-0 items-center gap-[0.35rem]">
		<Input
			class="min-w-0 flex-1"
			id="openrouter-key"
			value={openRouter.keyValue}
			type={openRouter.showKey ? 'text' : 'password'}
			readonly={openRouter.storedKeyReadonly}
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
			disabled={settings.locked || (!openRouter.hasStoredKey && !openRouter.replacingKey)}
		>
			{#if openRouter.showKey}<EyeOff size={14} />{:else}<Eye size={14} />{/if}
		</Button>
		{#if openRouter.hasStoredKey && !openRouter.replacingKey}
			<Button
				variant="ghost"
				size="icon-sm"
				onclick={() => void openRouter.copyKey()}
				aria-label="Copy saved OpenRouter key"
				disabled={settings.locked}
			>
				<Copy size={14} />
			</Button>
		{/if}
	</div>
	{#if openRouter.hasStoredKey}
		<div
			class="col-start-2 flex flex-wrap items-center justify-end gap-[0.35rem] max-[800px]:col-start-1 max-[800px]:justify-start"
		>
			<Button
				variant="ghost"
				size="sm"
				disabled={settings.locked}
				onclick={() => openRouter.beginKeyReplacement()}>Replace saved key</Button
			>
			<Button
				variant="ghost"
				size="sm"
				disabled={settings.locked}
				onclick={() => openRouter.toggleClearKey()}
			>
				{openRouter.clearKey ? 'Keep stored key' : 'Clear stored key'}
			</Button>
		</div>
	{/if}
	<div
		class={cn(
			'col-start-2 flex min-h-[1.8rem] items-center gap-2 text-[0.6875rem] text-ink-muted max-[800px]:col-start-1',
			openRouter.connectionState === 'success' && 'text-ink',
			(openRouter.connectionState === 'failed' ||
				settings.app.openRouterCredentialState === 'invalid') &&
				'text-scarlet-lamp'
		)}
		aria-live="polite"
		aria-busy={openRouter.connectionState === 'testing'}
	>
		{#if openRouter.connectionState === 'testing'}
			<span class="inline-flex animate-spin motion-reduce:animate-none"
				><RefreshCw size={14} /></span
			>
			<span>Checking key with OpenRouter…</span>
		{:else if openRouter.connectionState === 'success'}
			<ShieldCheck size={15} />
			<strong class="text-[inherit] text-ink">API key verified · saving locally</strong>
		{:else if openRouter.connectionState === 'failed'}
			<AlertTriangle size={15} />
			<span>Not saved. OpenRouter rejected this key. {openRouter.credentialMessage}</span>
		{:else if openRouter.keyDirty && openRouter.key.trim() !== ''}
			<span>{openRouter.key.trim().length} / 73 characters · not saved until verified</span>
		{:else if openRouter.credentialMessage}
			<span>{openRouter.credentialMessage}</span>
		{:else if openRouter.hasStoredKey && settings.app.openRouterCredentialState === 'checking'}
			<span class="inline-flex animate-spin motion-reduce:animate-none"
				><RefreshCw size={14} /></span
			>
			<span>Checking the saved API key…</span>
		{:else if openRouter.hasStoredKey && settings.app.openRouterCredentialState === 'invalid'}
			<AlertTriangle size={15} />
			<span>Saved API key rejected · replace it or retry verification</span>
		{:else if openRouter.hasStoredKey && settings.app.openRouterReady}
			<ShieldCheck size={15} />
			<span>Saved API key · verified and ready</span>
		{:else if openRouter.hasStoredKey}
			<ShieldCheck size={15} />
			<span>Saved API key · verification required</span>
		{:else}
			<span>Keys are verified automatically when all 73 characters are present.</span>
		{/if}
	</div>
	{#if openRouter.connectionState === 'success' && openRouter.keyReport}
		<p
			class="col-start-2 -mt-[0.55rem] text-[0.6875rem] leading-normal text-ink-muted max-[800px]:col-start-1"
		>
			{openRouter.keyReport.label ?? 'OpenRouter key'} ·
			{openRouter.keyReport.limit_remaining == null
				? 'No credit limit reported'
				: `${openRouter.keyReport.limit_remaining.toFixed(4)} credits remaining`}
		</p>
	{/if}
</SettingsRow>

<p
	class="m-0 border-l border-scarlet bg-plate px-[0.8rem] py-[0.65rem] text-[0.6875rem] leading-[1.55] text-ink-muted"
>
	Audio is sent to OpenRouter only after recording stops. OpenRouter mode uses batch transcription.
</p>

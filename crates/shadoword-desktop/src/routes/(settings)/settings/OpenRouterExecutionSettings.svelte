<script lang="ts">
	import { Eye, EyeOff, RefreshCw } from '@lucide/svelte';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Select } from '$lib/components/ui/select';
	import { StatusIndicator } from '$lib/components/ui/status-indicator';
	import type { RuntimeState } from '$lib/types';
	import SettingsRow from '../SettingsRow.svelte';
	import { getSettingsContext } from '../_state/context.svelte';
	import { cn } from '$lib/utils';
	import { onMount } from 'svelte';

	const settings = getSettingsContext();
	const openRouter = settings.openRouter;
	const rowClass = 'grid-cols-[minmax(0,1fr)_var(--control-width)] max-[800px]:grid-cols-1';
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

	let verifiedReadout = $derived.by(() => {
		const report = openRouter.keyReport;
		if (!report) return '';
		if (report.limit_remaining !== null) return `${formatCredits(report.limit_remaining)} left`;
		if (report.limit === null) return report.is_free_tier ? 'free' : 'unmetered';
		return '';
	});

	function formatCredits(value: number) {
		return value >= 100 ? `$${Math.round(value)}` : `$${value.toFixed(2)}`;
	}

	type KeyStatus = { state: RuntimeState; label: string; readout: string; title: string };

	let keyStatus = $derived.by((): KeyStatus | null => {
		const blank = { readout: '', title: '' };
		if (openRouter.connectionState === 'testing')
			return { state: 'loading', label: 'Testing', ...blank };
		if (openRouter.connectionState === 'success')
			return { state: 'ready', label: 'Verified', readout: verifiedReadout, title: '' };
		if (openRouter.connectionState === 'failed')
			return {
				state: 'offline',
				label: 'Rejected',
				readout: '',
				title: openRouter.credentialMessage
			};
		if (openRouter.credentialMessage)
			return {
				state: 'warning',
				label: 'Unavailable',
				readout: '',
				title: openRouter.credentialMessage
			};
		if (!openRouter.hasStoredKey) return null;
		if (settings.app.openRouterCredentialState === 'checking')
			return { state: 'loading', label: 'Checking', ...blank };
		if (settings.app.openRouterCredentialState === 'invalid')
			return { state: 'offline', label: 'Rejected', ...blank };
		if (settings.app.openRouterReady) return { state: 'ready', label: 'Verified', ...blank };
		return { state: 'warning', label: 'Not verified', ...blank };
	});

	onMount(() => {
		if (settings.app.openRouterModelsState === 'idle') {
			void settings.app.refreshOpenRouterModels();
		}
		void openRouter.loadSavedKey();
	});
</script>

<SettingsRow class={rowClass}>
	<div>
		<label for="openrouter-model" class="text-xs font-[570] text-ink">Transcription model</label>
		<p class="mt-[0.2rem] text-[0.6875rem] leading-[1.45] text-ink-muted">
			Use an OpenRouter model with transcription output.
		</p>
	</div>
	<div class="flex w-[var(--control-width)] max-w-full min-w-0 flex-wrap items-center gap-2">
		<Select
			id="openrouter-model"
			class="min-w-0 flex-1 text-left"
			contentClass="max-h-[22rem] w-[min(31rem,var(--bits-select-anchor-width))]"
			value={openRouter.model}
			onValueChange={(value) => openRouter.setModel(value)}
			options={modelOptions}
			disabled={settings.locked || settings.app.openRouterModelsState === 'loading'}
			ariaLabel="OpenRouter transcription model"
			ariaBusy={settings.app.openRouterModelsState === 'loading'}
			menuLabel={`${settings.app.openRouterModels.length} transcription models`}
		/>
	</div>
	{#if selectedModel}
		<p class="col-span-full text-[0.6875rem] leading-normal text-ink-muted">
			{selectedModel.description}
		</p>
	{:else if settings.app.openRouterModelsError}
		<p class="col-span-full text-[0.6875rem] text-scarlet-lamp" role="alert">
			{settings.app.openRouterModelsError}
		</p>
	{/if}
</SettingsRow>

<SettingsRow class={rowClass}>
	<div>
		<label for="openrouter-key" class="text-xs font-[570] text-ink">OpenRouter API key</label>
		<p class="mt-[0.2rem] text-[0.6875rem] leading-[1.45] text-ink-muted">
			Stored only in the native Shadoword desktop configuration.
		</p>
	</div>
	<div class="relative w-[var(--control-width)] max-w-full min-w-0">
		<Input
			class="pr-8"
			id="openrouter-key"
			value={openRouter.keyValue}
			type={openRouter.showKey ? 'text' : 'password'}
			placeholder="Enter an OpenRouter API key"
			disabled={settings.locked}
			oninput={(event) => openRouter.setKey(event.currentTarget.value)}
		/>
		<Button
			variant="ghost"
			size="icon-xs"
			class="absolute top-1/2 right-1 -translate-y-1/2 text-ink-dim hover:text-ink"
			onclick={() => openRouter.toggleKeyVisibility()}
			aria-label={openRouter.showKey ? 'Hide OpenRouter key' : 'Show OpenRouter key'}
			disabled={settings.locked || openRouter.keyValue === ''}
		>
			{#if openRouter.showKey}
				<EyeOff class="size-3.5" />
			{:else}
				<Eye class="size-3.5" />
			{/if}
		</Button>
	</div>
	<div class="col-span-2 flex items-center justify-end gap-3 max-[800px]:col-span-1">
		<div
			class="flex min-h-7 min-w-0 items-center"
			aria-live="polite"
			aria-busy={openRouter.connectionState === 'testing'}
		>
			{#if keyStatus}
				<span
					class={cn(
						'inline-flex min-w-0 items-center gap-[0.45rem] border bg-plate py-[0.2rem] pr-[0.5rem] pl-[0.45rem] uppercase',
						'[&_.status]:text-[0.625rem] [&_.status]:tracking-[0.09em]',
						keyStatus.state === 'ready' && 'border-line-strong',
						keyStatus.state === 'offline' && 'border-scarlet',
						(keyStatus.state === 'warning' || keyStatus.state === 'loading') && 'border-line'
					)}
					title={keyStatus.title || undefined}
				>
					<StatusIndicator compact state={keyStatus.state} label={keyStatus.label} />
					{#if keyStatus.readout}
						<span
							class="min-w-0 shrink-0 border-l border-line pl-[0.45rem] font-mono text-[0.625rem] tracking-[0.09em] text-ink-dim tabular-nums"
						>
							{keyStatus.readout}
						</span>
					{/if}
				</span>
			{/if}
		</div>
		<Button
			variant="outline"
			size="sm"
			class="shrink-0"
			onclick={() => void openRouter.testKey()}
			disabled={settings.locked || !openRouter.canTestKey}
			aria-busy={openRouter.connectionState === 'testing'}
		>
			<span
				class={cn(
					'inline-flex',
					openRouter.connectionState === 'testing' && 'animate-spin motion-reduce:animate-none'
				)}
			>
				<RefreshCw size={14} />
			</span>
			{openRouter.connectionState === 'testing' ? 'Testing…' : 'Test key'}
		</Button>
	</div>
</SettingsRow>

<p
	class="m-0 border-l border-scarlet bg-plate px-[0.8rem] py-[0.65rem] text-[0.6875rem] leading-[1.55] text-ink-muted"
>
	With segmented delivery enabled, OpenRouter receives each pause-separated WAV in order. Otherwise,
	it receives one WAV after recording stops.
</p>

<script lang="ts">
	import { CircleHelp, Copy, Eye, EyeOff, RefreshCw } from '@lucide/svelte';
	import { StatusIndicator } from '$lib/components/ui/status-indicator';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import * as Tooltip from '$lib/components/ui/tooltip';
	import SettingsRow from '../SettingsRow.svelte';
	import { getSettingsContext } from '../_state/context.svelte';
	import { cn } from '$lib/utils';

	const settings = getSettingsContext();
	const remote = settings.remote;
	const rowClass = 'grid-cols-[minmax(0,1fr)_var(--control-width)] max-[800px]:grid-cols-1';
</script>

<SettingsRow class={rowClass}>
	<div>
		<label for="endpoint" class="text-xs font-[570] text-ink">API endpoint</label>
		<p class="mt-[0.2rem] text-[0.6875rem] leading-[1.45] text-ink-muted">
			Use HTTPS outside an encrypted private network.
		</p>
	</div>
	<div class="relative w-[var(--control-width)] max-w-full min-w-0">
		<Input
			class="pr-10"
			id="endpoint"
			value={remote.endpoint}
			disabled={settings.locked}
			oninput={(event) => remote.setEndpoint(event.currentTarget.value)}
		/>
		<Tooltip.Provider delayDuration={200}>
			<Tooltip.Root>
				<Tooltip.Trigger
					type="button"
					class="absolute top-1/2 right-2 z-10 inline-flex size-6 -translate-y-1/2 cursor-help items-center justify-center border-0 bg-transparent text-ink-dim transition-colors hover:text-scarlet-lamp focus-visible:outline focus-visible:outline-1 focus-visible:outline-offset-1 focus-visible:outline-scarlet"
					aria-label="Default Shadoword API port: 47813"
				>
					<CircleHelp size={14} aria-hidden="true" />
				</Tooltip.Trigger>
				<Tooltip.Content
					side="top"
					sideOffset={7}
					class="rounded-none border border-line bg-raised px-2.5 py-2 font-mono text-[0.6875rem] text-ink shadow-[4px_4px_0_var(--void)]"
					arrowClasses="bg-raised fill-raised"
					role="tooltip"
				>
					Default Shadoword API port: <strong class="text-scarlet-lamp">47813</strong>
				</Tooltip.Content>
			</Tooltip.Root>
		</Tooltip.Provider>
	</div>
</SettingsRow>
<SettingsRow class={rowClass}>
	<div>
		<label for="token" class="text-xs font-[570] text-ink">Bearer token</label>
		<p class="mt-[0.2rem] text-[0.6875rem] leading-[1.45] text-ink-muted">
			An admin token is required for desktop management and is stored privately.
		</p>
	</div>
	<div class="flex w-[var(--control-width)] max-w-full min-w-0 items-center gap-[0.35rem]">
		<Input
			class="min-w-0 flex-1"
			id="token"
			value={remote.tokenValue}
			type={remote.showToken ? 'text' : 'password'}
			readonly={remote.storedTokenReadonly}
			placeholder={settings.app.settings?.remote_token_configured
				? 'Stored token unchanged'
				: 'No token configured'}
			disabled={settings.locked}
			oninput={(event) => remote.setToken(event.currentTarget.value)}
		/>
		<Button
			variant="ghost"
			size="icon-sm"
			onclick={() => remote.toggleTokenVisibility()}
			aria-label={remote.showToken ? 'Hide token' : 'Show token'}
			disabled={settings.locked || (!remote.hasStoredToken && !remote.tokenDirty)}
		>
			{#if remote.showToken}<EyeOff size={14} />{:else}<Eye size={14} />{/if}
		</Button>
		{#if remote.hasStoredToken && !remote.tokenDirty}
			<Button
				variant="ghost"
				size="icon-sm"
				disabled={settings.locked}
				onclick={() => void remote.copyToken()}
				aria-label="Copy saved bearer token"
			>
				<Copy size={14} />
			</Button>
		{/if}
	</div>
	{#if remote.hasStoredToken}
		<div
			class="col-start-2 flex flex-wrap items-center justify-end gap-[0.35rem] max-[800px]:col-start-1 max-[800px]:justify-start"
		>
			<Button
				variant="ghost"
				size="sm"
				disabled={settings.locked}
				onclick={() => remote.beginTokenReplacement()}>Replace saved token</Button
			>
			<Button
				variant="ghost"
				size="sm"
				disabled={settings.locked}
				onclick={() => remote.toggleClearToken()}
			>
				{remote.clearToken ? 'Keep stored token' : 'Clear stored token'}
			</Button>
		</div>
	{/if}
	{#if remote.credentialMessage}
		<span
			class="col-start-2 text-[0.6875rem] text-ink-muted max-[800px]:col-start-1"
			aria-live="polite"
		>
			{remote.credentialMessage}
		</span>
	{/if}
</SettingsRow>
<SettingsRow class="grid-cols-1">
	<div class="flex flex-wrap items-center gap-2">
		<Button
			variant="outline"
			size="sm"
			onclick={() => void remote.testConnection()}
			disabled={settings.locked || remote.connectionState === 'testing'}
			aria-busy={remote.connectionState === 'testing'}
		>
			<span
				class={cn(
					'inline-flex',
					remote.connectionState === 'testing' && 'animate-spin motion-reduce:animate-none'
				)}><RefreshCw size={14} /></span
			>
			{remote.connectionState === 'testing' ? 'Testing…' : 'Test connection'}
		</Button>
		<div
			class="flex min-h-7 min-w-0 items-center"
			aria-live="polite"
			aria-busy={remote.connectionState === 'testing'}
		>
			{#if remote.connectionState === 'testing'}
				<span class="text-[0.6875rem] text-ink-muted">Checking endpoint and credentials…</span>
			{:else if remote.connectionState === 'success'}
				<StatusIndicator label={settings.app.connectionMessage ?? 'Connected'} />
			{:else if remote.connectionState === 'failed'}
				<StatusIndicator state="offline" label="Connection failed" />
			{:else if remote.verificationRequired}
				<span class="text-[0.6875rem] text-ink-muted">
					Test this endpoint and token together. Changes save after a successful test.
				</span>
			{/if}
		</div>
	</div>
</SettingsRow>

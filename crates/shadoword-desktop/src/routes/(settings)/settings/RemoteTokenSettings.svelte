<script lang="ts">
	import { Check, Copy, KeyRound, RefreshCw, Trash2 } from '@lucide/svelte';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Select } from '$lib/components/ui/select';
	import SettingsRow from '../SettingsRow.svelte';
	import { getSettingsContext } from '../_state/context.svelte';
	import { cn } from '$lib/utils';

	const settings = getSettingsContext();
	const tokens = settings.remoteTokens;
	const rowClass = 'grid-cols-[minmax(0,1fr)_var(--control-width)] max-[800px]:grid-cols-1';
	const roleOptions = [
		{ value: 'user', label: 'User', detail: 'Transcribe audio only' },
		{ value: 'admin', label: 'Admin', detail: 'Transcribe and manage the daemon' }
	] as const;

	// The daemon refuses to revoke its last admin token, so the button is disabled
	// rather than left to fail: the rule is easier to read as a greyed control than
	// as an error that only appears after the operator has committed to the action.
	let adminCount = $derived(tokens.tokens.filter((token) => token.role === 'admin').length);
</script>

<SettingsRow class="grid-cols-1">
	<div class="flex flex-wrap items-center justify-between gap-2">
		<div>
			<span class="text-xs font-[570] text-ink">Daemon tokens</span>
			<p class="mt-[0.2rem] text-[0.6875rem] leading-[1.45] text-ink-muted">
				Issued and revoked on the connected daemon. Changes apply without a restart.
			</p>
		</div>
		<Button
			variant="outline"
			size="sm"
			onclick={() => void tokens.load()}
			disabled={settings.locked || tokens.busy}
			aria-busy={tokens.busy}
		>
			<span class={cn('inline-flex', tokens.busy && 'animate-spin motion-reduce:animate-none')}>
				<RefreshCw size={14} />
			</span>
			{tokens.loaded ? 'Refresh' : 'Load tokens'}
		</Button>
	</div>
</SettingsRow>

{#if tokens.loaded}
	<SettingsRow class="grid-cols-1">
		{#if tokens.tokens.length > 0}
			<ul class="m-0 grid list-none gap-px border border-line bg-line p-0">
				{#each tokens.tokens as token (token.name)}
					<li class="flex items-center justify-between gap-3 bg-plate px-[0.85rem] py-[0.6rem]">
						<div class="flex min-w-0 items-center gap-[0.6rem]">
							<KeyRound size={14} class="shrink-0 text-ink-muted" />
							<span class="truncate font-mono text-[0.75rem] text-ink">{token.name}</span>
							<Badge variant="outline">{token.role === 'admin' ? 'Admin' : 'User'}</Badge>
						</div>
						<Button
							variant="ghost"
							size="icon-sm"
							onclick={() => void tokens.revoke(token.name)}
							disabled={settings.locked ||
								tokens.busy ||
								(token.role === 'admin' && adminCount === 1)}
							aria-label={token.role === 'admin' && adminCount === 1
								? `Cannot revoke ${token.name}: it is the last admin token`
								: `Revoke ${token.name}`}
						>
							<Trash2 size={14} />
						</Button>
					</li>
				{/each}
			</ul>
		{:else}
			<p
				class="m-0 border border-dashed border-line-strong px-4 py-3 text-[0.6875rem] text-ink-dim"
			>
				This daemon has no tokens. It accepts every caller that can reach it.
			</p>
		{/if}
	</SettingsRow>

	<SettingsRow class={rowClass}>
		<div>
			<label for="token-name" class="text-xs font-[570] text-ink">Issue a token</label>
			<p class="mt-[0.2rem] text-[0.6875rem] leading-[1.45] text-ink-muted">
				The secret is shown once. The daemon keeps only a hash of it.
			</p>
		</div>
		<div class="grid w-[var(--control-width)] max-w-full min-w-0 gap-[0.35rem]">
			<Input
				id="token-name"
				value={tokens.name}
				placeholder="Token name"
				disabled={settings.locked || tokens.busy}
				oninput={(event) => (tokens.name = event.currentTarget.value)}
			/>
			<Select
				value={tokens.role}
				options={roleOptions}
				disabled={settings.locked || tokens.busy}
				ariaLabel="Token role"
				onValueChange={(value) => (tokens.role = value === 'admin' ? 'admin' : 'user')}
			/>
			<Button
				variant="outline"
				size="sm"
				onclick={() => void tokens.create()}
				disabled={settings.locked || !tokens.canCreate}
			>
				Issue token
			</Button>
		</div>
	</SettingsRow>
{/if}

{#if tokens.issued}
	<SettingsRow class="grid-cols-1">
		<div class="grid gap-[0.6rem] border border-scarlet bg-raised px-4 py-[0.85rem]">
			<div class="flex flex-wrap items-center justify-between gap-2">
				<strong class="text-xs font-[570] text-ink">
					Copy {tokens.issued.name} now — it is not shown again
				</strong>
				<div class="flex gap-[0.35rem]">
					<Button variant="outline" size="sm" onclick={() => void tokens.copyIssued()}>
						{#if tokens.copied}<Check size={14} />Copied{:else}<Copy size={14} />Copy{/if}
					</Button>
					<Button variant="ghost" size="sm" onclick={() => tokens.dismissIssued()}>Dismiss</Button>
				</div>
			</div>
			<code class="font-mono text-[0.75rem] [overflow-wrap:anywhere] text-ink">
				{tokens.issued.token}
			</code>
		</div>
	</SettingsRow>
{/if}

{#if tokens.error}
	<SettingsRow class="grid-cols-1">
		<span class="text-[0.6875rem] text-scarlet-lamp" role="alert">{tokens.error}</span>
	</SettingsRow>
{/if}

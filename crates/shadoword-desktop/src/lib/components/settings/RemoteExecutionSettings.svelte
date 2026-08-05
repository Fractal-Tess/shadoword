<script lang="ts">
	import { Eye, EyeOff, RefreshCw } from '@lucide/svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { getSettingsContext } from '$lib/settings/context.svelte';

	const settings = getSettingsContext();
	const remote = settings.remote;
</script>

<div class="stacked-setting">
	<div>
		<label for="endpoint">API endpoint</label>
		<p>Use HTTPS outside an encrypted private network.</p>
	</div>
	<Input
		id="endpoint"
		value={remote.endpoint}
		disabled={settings.locked}
		oninput={(event) => remote.setEndpoint(event.currentTarget.value)}
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
			value={remote.token}
			type={remote.showToken ? 'text' : 'password'}
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
			disabled={settings.locked}
		>
			{#if remote.showToken}<EyeOff size={14} />{:else}<Eye size={14} />{/if}
		</Button>
	</div>
	{#if settings.app.settings?.remote_token_configured}
		<Button
			variant="ghost"
			size="sm"
			disabled={settings.locked}
			onclick={() => remote.toggleClearToken()}
		>
			{remote.clearToken ? 'Keep stored token' : 'Clear stored token'}
		</Button>
	{/if}
</div>
<div class="connection-row">
	<Button
		variant="outline"
		size="sm"
		onclick={() => void remote.testConnection()}
		disabled={settings.locked || remote.connectionState === 'testing'}
	>
		<span class:spin={remote.connectionState === 'testing'}><RefreshCw size={14} /></span>
		{remote.connectionState === 'testing' ? 'Testing…' : 'Test connection'}
	</Button>
	{#if remote.connectionState === 'success'}
		<StatusPill label={settings.app.connectionMessage ?? 'Connected'} />
	{:else if remote.connectionState === 'failed'}
		<StatusPill state="offline" label="Connection failed" />
	{:else if remote.verificationRequired}
		<span class="verification-note">
			Test this endpoint and token together before they are saved.
		</span>
	{/if}
</div>

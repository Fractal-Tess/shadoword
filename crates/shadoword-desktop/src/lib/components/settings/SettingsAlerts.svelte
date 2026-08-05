<script lang="ts">
	import { AlertTriangle, RefreshCw } from '@lucide/svelte';
	import { Button } from '$lib/components/ui/button';
	import { getSettingsContext } from '$lib/settings/context.svelte';

	const settings = getSettingsContext();
</script>

{#if settings.persistence.saveState === 'failed'}
	<div class="save-error" role="alert">
		<AlertTriangle size={17} />
		<div>
			<strong>Shadoword could not save desktop.json</strong>
			<span>{settings.persistence.error}</span>
		</div>
		<Button
			variant="outline"
			size="sm"
			onclick={() => void settings.persistence.save()}
			disabled={settings.locked}>Retry save</Button
		>
	</div>
{/if}

{#if settings.app.captureLocked}
	<div class="save-error capture-lock" role="status">
		<RefreshCw size={17} />
		<div>
			<strong>
				Settings are locked during {settings.app.processing ? 'finalization' : 'recording'}
			</strong>
			<span>Stop the active session before changing native configuration.</span>
		</div>
	</div>
{/if}

<script lang="ts">
	import { ArrowRight, SlidersHorizontal } from '@lucide/svelte';
	import OpenRouterExecutionSettings from '$lib/components/settings/OpenRouterExecutionSettings.svelte';
	import RemoteExecutionSettings from '$lib/components/settings/RemoteExecutionSettings.svelte';
	import { Button } from '$lib/components/ui/button';
	import { getSettingsContext } from '$lib/settings/context.svelte';

	const settings = getSettingsContext();
</script>

<section id="runtime">
	<header>
		<div class="section-icon"><SlidersHorizontal size={16} /></div>
		<div>
			<h2 class="display-legend">Runtime</h2>
			<p>Configure the execution target selected for the entire desktop.</p>
		</div>
	</header>
	<div class="setting-list">
		{#if settings.mode === 'local'}
			<div class="setting-row local-runtime-row">
				<div>
					<span class="setting-label">Active model</span>
					<p>Model weights stay on this machine.</p>
				</div>
				<div class="runtime-summary">
					<strong>{settings.localModelName}</strong>
					<span>
						{settings.activeRuntime?.whisper_accelerator ?? 'CPU'} · {settings.poolSummary}
					</span>
				</div>
			</div>
			<div class="setting-row">
				<div>
					<span class="setting-label">Models and execution pool</span>
					<p>Manage model downloads, accelerator affinity, and worker topology.</p>
				</div>
				<Button variant="outline" size="sm" onclick={() => settings.navigate('models')}>
					Open runtime <ArrowRight size={14} />
				</Button>
			</div>
		{:else if settings.mode === 'remote'}
			<RemoteExecutionSettings />
		{:else}
			<OpenRouterExecutionSettings />
		{/if}
	</div>
</section>

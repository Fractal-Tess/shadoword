<script lang="ts">
	import { SlidersHorizontal } from '@lucide/svelte';
	import SettingsPanel from '../SettingsPanel.svelte';
	import SettingsRow from '../SettingsRow.svelte';
	import SettingsSection from '../SettingsSection.svelte';
	import { getSettingsContext } from '../_state/context.svelte';
	import OpenRouterExecutionSettings from './OpenRouterExecutionSettings.svelte';
	import RemoteExecutionSettings from './RemoteExecutionSettings.svelte';
	import ExecutionRuntimeSettings from './ExecutionRuntimeSettings.svelte';

	const settings = getSettingsContext();
</script>

<SettingsSection
	id="runtime"
	title="Runtime"
	description="Configure connectivity, hardware, and worker behavior for the selected target."
>
	{#snippet icon()}<SlidersHorizontal size={16} aria-hidden="true" />{/snippet}
	{#if settings.mode === 'local'}
		<SettingsPanel class="mb-4">
			<SettingsRow>
				<div>
					<span class="text-xs font-[570] text-ink">Active model</span>
					<p class="mt-[0.2rem] text-[0.6875rem] leading-[1.45] text-ink-muted">
						Model weights and transcription stay on this machine.
					</p>
				</div>
				<div
					class="grid justify-items-end gap-[0.18rem] text-right max-[800px]:justify-items-start max-[800px]:text-left"
				>
					<strong class="text-xs font-[590] text-ink">{settings.localModelName}</strong>
					<span class="font-mono text-[0.6875rem] text-ink-muted">
						{settings.activeRuntime?.whisper_accelerator ?? 'CPU'} · {settings.poolSummary}
					</span>
				</div>
			</SettingsRow>
		</SettingsPanel>
		<ExecutionRuntimeSettings />
	{:else if settings.mode === 'remote'}
		<SettingsPanel class="mb-4">
			<RemoteExecutionSettings />
		</SettingsPanel>
		<ExecutionRuntimeSettings />
	{:else}
		<SettingsPanel>
			<OpenRouterExecutionSettings />
		</SettingsPanel>
	{/if}
</SettingsSection>

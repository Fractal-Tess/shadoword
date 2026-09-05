<script lang="ts">
	import { Keyboard } from '@lucide/svelte';
	import { Select } from '$lib/components/ui/select';
	import SettingsPanel from '../SettingsPanel.svelte';
	import SettingsRow from '../SettingsRow.svelte';
	import SettingsSection from '../SettingsSection.svelte';
	import { getSettingsContext } from '../_state/context.svelte';
	import { PCM_FORMAT_OPTIONS } from '../_state/options';
	import MicrophoneSettings from './MicrophoneSettings.svelte';
	import ShortcutSettings from './ShortcutSettings.svelte';

	const settings = getSettingsContext();
	const form = settings.form;
</script>

<SettingsSection
	id="capture"
	title="Capture"
	description="Select an input and a shortcut that works anywhere on the desktop."
>
	{#snippet icon()}<Keyboard size={16} aria-hidden="true" />{/snippet}
	<SettingsPanel>
		<MicrophoneSettings />
		{#if settings.mode === 'remote' && form.transcriptionMode === 'streaming'}
			<SettingsRow>
				<div>
					<label for="streaming-pcm-format" class="text-xs font-[570] text-ink">
						Streaming PCM precision
					</label>
					<p class="mt-[0.2rem] text-[0.6875rem] leading-[1.45] text-ink-muted">
						Choose the protocol-v3 wire format used for remote live audio.
					</p>
				</div>
				<Select
					id="streaming-pcm-format"
					class="w-[var(--control-width)] max-w-full"
					value={form.streamingPcmFormat}
					onValueChange={(value) => form.setStreamingPcmFormat(value)}
					options={PCM_FORMAT_OPTIONS}
					disabled={settings.locked}
					ariaLabel="Streaming PCM precision"
				/>
			</SettingsRow>
		{/if}
		<ShortcutSettings />
	</SettingsPanel>
</SettingsSection>

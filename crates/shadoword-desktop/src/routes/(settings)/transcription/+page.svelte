<script lang="ts">
	import { Captions } from '@lucide/svelte';
	import { Switch } from '$lib/components/ui/switch';
	import SettingsPanel from '../SettingsPanel.svelte';
	import SettingsRow from '../SettingsRow.svelte';
	import SettingsSection from '../SettingsSection.svelte';
	import { getSettingsContext } from '../_state/context.svelte';

	const settings = getSettingsContext();
	const form = settings.form;
</script>

<SettingsSection
	id="transcription"
	title="Transcription"
	description="Recognition and segmentation preferences for the active runtime."
>
	{#snippet icon()}<Captions size={16} aria-hidden="true" />{/snippet}
	<SettingsPanel>
		<SettingsRow>
			<div>
				<label for="english-only" class="text-xs font-[570] text-ink">English recognition</label>
				<p class="mt-[0.2rem] text-[0.6875rem] leading-[1.45] text-ink-muted">
					Constrain recognition to English instead of detecting language.
				</p>
			</div>
			<Switch
				id="english-only"
				checked={form.englishOnly}
				onCheckedChange={(checked) => form.setEnglishOnly(checked)}
				disabled={settings.locked}
				aria-label="English recognition"
			/>
		</SettingsRow>
		<SettingsRow>
			<div>
				<label for="streaming-segments" class="text-xs font-[570] text-ink">
					Submit pause-separated segments
				</label>
				<p class="mt-[0.2rem] text-[0.6875rem] leading-[1.45] text-ink-muted">
					{settings.mode === 'open_router'
						? 'Use on-device voice activity detection to submit each completed WAV segment in order.'
						: 'Commit pause-separated segments while recording, or transcribe once after stop.'}
				</p>
			</div>
			<Switch
				id="streaming-segments"
				checked={form.transcriptionMode === 'streaming'}
				onCheckedChange={(checked) => form.setTranscriptionMode(checked ? 'streaming' : 'batch')}
				disabled={settings.locked}
				aria-label="Submit pause-separated segments"
			/>
		</SettingsRow>
	</SettingsPanel>
</SettingsSection>

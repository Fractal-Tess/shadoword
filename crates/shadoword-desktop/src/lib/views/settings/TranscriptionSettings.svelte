<script lang="ts">
	import { Captions } from '@lucide/svelte';
	import { Switch } from '$lib/components/ui/switch';
	import { getSettingsContext } from '$lib/settings/context.svelte';

	const settings = getSettingsContext();
	const form = settings.form;
</script>

<section id="transcription">
	<header>
		<div class="section-icon"><Captions size={16} /></div>
		<div>
			<h2 class="display-legend">Transcription</h2>
			<p>Recognition and segmentation preferences for the active runtime.</p>
		</div>
	</header>
	<div class="setting-list">
		<div class="setting-row">
			<div>
				<label for="english-only">English recognition</label>
				<p>Constrain recognition to English instead of detecting language.</p>
			</div>
			<Switch
				id="english-only"
				checked={form.englishOnly}
				onCheckedChange={(checked) => form.setEnglishOnly(checked)}
				disabled={settings.locked}
				aria-label="English recognition"
			/>
		</div>
		{#if settings.mode === 'open_router'}
			<div class="setting-row">
				<div>
					<span class="setting-label">Transcription delivery</span>
					<p>OpenRouter receives one WAV file after recording stops.</p>
				</div>
				<div class="batch-only" aria-label="OpenRouter transcription mode: batch only">
					<strong>Batch only</strong>
					<span>Streaming is not available</span>
				</div>
			</div>
		{:else}
			<div class="setting-row">
				<div>
					<label for="streaming-segments">Stream pause-separated segments</label>
					<p>Commit pause-separated segments while recording, or transcribe once after stop.</p>
				</div>
				<Switch
					id="streaming-segments"
					checked={form.transcriptionMode === 'streaming'}
					onCheckedChange={(checked) => form.setTranscriptionMode(checked ? 'streaming' : 'batch')}
					disabled={settings.locked}
					aria-label="Stream pause-separated segments"
				/>
			</div>
		{/if}
	</div>
</section>

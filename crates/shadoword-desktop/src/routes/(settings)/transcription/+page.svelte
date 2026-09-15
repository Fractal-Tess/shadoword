<script lang="ts">
	import { Captions, X } from '@lucide/svelte';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Switch } from '$lib/components/ui/switch';
	import SettingsPanel from '../SettingsPanel.svelte';
	import SettingsRow from '../SettingsRow.svelte';
	import SettingsSection from '../SettingsSection.svelte';
	import { getSettingsContext } from '../_state/context.svelte';

	const settings = getSettingsContext();
	const form = settings.form;

	let vocabularyDraft = $state('');
	let vocabularyFilter = $state('');
	let vocabularyInput = $state<HTMLInputElement | null>(null);
	const vocabularyWords = $derived([
		...new Set(
			form.customVocabulary
				.split(/[,\r\n]+/)
				.map((word) => word.trim())
				.filter(Boolean)
		)
	]);
	const filterQuery = $derived(vocabularyFilter.trim().toLowerCase());
	const visibleWords = $derived(
		vocabularyWords.filter((word) => word.toLowerCase().includes(filterQuery))
	);

	function addVocabularyWords(value = vocabularyDraft) {
		if (settings.locked) return;
		const additions = value.split(/[\s,]+/).filter(Boolean);
		if (additions.length === 0) {
			vocabularyDraft = '';
			return;
		}
		const words = [...new Set([...vocabularyWords, ...additions])];
		if (words.length !== vocabularyWords.length) {
			form.setCustomVocabulary(words.join('\n'));
		}
		vocabularyDraft = '';
		vocabularyInput?.focus();
	}

	function handleVocabularyKeydown(event: KeyboardEvent) {
		if (event.isComposing || event.keyCode === 229) return;
		if (event.key === ' ' || event.key === 'Enter' || event.key === ',') {
			event.preventDefault();
			addVocabularyWords();
		}
	}

	function handleVocabularyPaste(event: ClipboardEvent & { currentTarget: HTMLInputElement }) {
		const text = event.clipboardData?.getData('text/plain');
		if (!text || !/[\s,]/.test(text) || settings.locked) return;
		event.preventDefault();
		const input = event.currentTarget;
		const start = input.selectionStart ?? input.value.length;
		const end = input.selectionEnd ?? start;
		addVocabularyWords(input.value.slice(0, start) + text + input.value.slice(end));
	}

	function removeVocabularyWord(word: string) {
		if (settings.locked) return;
		form.setCustomVocabulary(vocabularyWords.filter((entry) => entry !== word).join('\n'));
		vocabularyInput?.focus();
	}
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
		<SettingsRow class="grid-cols-1 gap-3">
			<div>
				<label for="custom-vocabulary" class="text-xs font-[570] text-ink">
					Custom vocabulary
				</label>
				<p
					id="custom-vocabulary-description"
					class="mt-[0.2rem] text-[0.6875rem] leading-[1.45] text-ink-muted"
				>
					Enter a correct spelling, then press Space or Enter to add it. Use × to remove a word.
				</p>
			</div>
			<div class="flex min-w-0 items-center gap-2">
				<Input
					id="custom-vocabulary"
					bind:ref={vocabularyInput}
					bind:value={vocabularyDraft}
					onkeydown={handleVocabularyKeydown}
					onpaste={handleVocabularyPaste}
					placeholder="Add a word, e.g. reranker"
					autocomplete="off"
					autocapitalize="off"
					spellcheck={false}
					disabled={settings.locked}
					aria-describedby="custom-vocabulary-description custom-vocabulary-help"
				/>
				<Button
					variant="outline"
					disabled={settings.locked || !vocabularyDraft.trim()}
					onclick={() => addVocabularyWords()}
				>
					Add
				</Button>
			</div>
			<div class="flex flex-col gap-2">
				<label for="vocabulary-filter" class="text-xs font-[570] text-ink">
					Filter vocabulary
				</label>
				<Input
					id="vocabulary-filter"
					type="search"
					bind:value={vocabularyFilter}
					placeholder="Find a saved word…"
					autocomplete="off"
					spellcheck={false}
					aria-controls="vocabulary-words"
				/>
				<p role="status" class="text-[0.6875rem] leading-[1.45] text-ink-muted">
					Showing {visibleWords.length} of {vocabularyWords.length} words
				</p>
				<ul
					id="vocabulary-words"
					aria-label="Custom vocabulary words"
					class="flex max-h-48 min-w-0 flex-wrap gap-2 overflow-y-auto p-1"
				>
					{#each visibleWords as word (word)}
						<li class="max-w-full min-w-0">
							<Badge variant="outline" class="h-auto max-w-full gap-1 py-1 pr-1">
								<span class="min-w-0 wrap-anywhere whitespace-normal">{word}</span>
								<Button
									variant="ghost"
									size="icon-xs"
									disabled={settings.locked}
									aria-label={`Remove ${word}`}
									onclick={() => removeVocabularyWord(word)}
								>
									<X aria-hidden="true" />
								</Button>
							</Badge>
						</li>
					{/each}
				</ul>
				{#if vocabularyWords.length === 0}
					<p class="text-[0.6875rem] leading-[1.45] text-ink-muted">No vocabulary added yet.</p>
				{:else if visibleWords.length === 0}
					<p class="text-[0.6875rem] leading-[1.45] text-ink-muted">No words match your filter.</p>
				{/if}
			</div>
			<p id="custom-vocabulary-help" class="text-[0.6875rem] leading-[1.45] text-ink-muted">
				These are recognition hints, not automatic replacements. Remove all words to disable.
				Filtering only changes this view; every saved word is still sent as a hint. Keep the list
				short; Whisper only uses a limited amount of vocabulary context.
				{#if settings.mode === 'remote'}
					This changes the vocabulary for all clients using this Shadoword API daemon.
				{:else if settings.mode === 'open_router'}
					OpenRouter support depends on the selected model and provider; some may ignore hints.
				{/if}
			</p>
		</SettingsRow>
	</SettingsPanel>
</SettingsSection>

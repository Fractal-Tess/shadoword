<script lang="ts">
	import { Send } from '@lucide/svelte';
	import { Select } from '$lib/components/ui/select';
	import { Input } from '$lib/components/ui/input';
	import { Switch } from '$lib/components/ui/switch';
	import SettingsPanel from '../SettingsPanel.svelte';
	import SettingsRow from '../SettingsRow.svelte';
	import SettingsSection from '../SettingsSection.svelte';
	import { getSettingsContext } from '../_state/context.svelte';
	import { PASTE_METHOD_OPTIONS, TRANSCRIPT_BOUNDARY_OPTIONS } from '../_state/options';

	const settings = getSettingsContext();
	const form = settings.form;
</script>

<SettingsSection
	id="output"
	title="Output"
	description="Control where completed transcript text is delivered."
>
	{#snippet icon()}<Send size={16} aria-hidden="true" />{/snippet}
	<SettingsPanel>
		<SettingsRow>
			<div>
				<label for="copy-final" class="text-xs font-[570] text-ink">Copy final transcript</label>
				<p class="mt-[0.2rem] text-[0.6875rem] leading-[1.45] text-ink-muted">
					Write the completed transcript to the system clipboard.
				</p>
			</div>
			<Switch
				id="copy-final"
				checked={form.copyFinal}
				onCheckedChange={(checked) => form.setCopyFinal(checked)}
				disabled={settings.locked}
				aria-label="Copy final transcript"
			/>
		</SettingsRow>
		<SettingsRow>
			<div>
				<label for="paste-method" class="text-xs font-[570] text-ink">
					Active-window delivery
				</label>
				<p class="mt-[0.2rem] text-[0.6875rem] leading-[1.45] text-ink-muted">
					Type directly or paste into the active window through the native host.
				</p>
			</div>
			<Select
				id="paste-method"
				value={form.pasteMethod}
				onValueChange={(value) => form.setPasteMethod(value)}
				options={PASTE_METHOD_OPTIONS}
				disabled={settings.locked}
				ariaLabel="Active-window delivery"
			/>
		</SettingsRow>
		<SettingsRow>
			<div>
				<span class="text-xs font-[570] text-ink">Transcript spacing</span>
				<p class="mt-[0.2rem] text-[0.6875rem] leading-[1.45] text-ink-muted">
					Add whitespace around each delivered transcript so consecutive recordings stay separate.
				</p>
			</div>
			<div class="grid grid-cols-2 gap-2 max-[520px]:grid-cols-1">
				<label class="grid gap-1 text-[0.6875rem] text-ink-muted" for="output-prefix">
					Before transcript
					<Select
						id="output-prefix"
						value={form.outputPrefix}
						onValueChange={(value) => form.setOutputPrefix(value)}
						options={TRANSCRIPT_BOUNDARY_OPTIONS}
						disabled={settings.locked}
						ariaLabel="Spacing before transcript"
					/>
				</label>
				<label class="grid gap-1 text-[0.6875rem] text-ink-muted" for="output-suffix">
					After transcript
					<Select
						id="output-suffix"
						value={form.outputSuffix}
						onValueChange={(value) => form.setOutputSuffix(value)}
						options={TRANSCRIPT_BOUNDARY_OPTIONS}
						disabled={settings.locked}
						ariaLabel="Spacing after transcript"
					/>
				</label>
			</div>
		</SettingsRow>
		{#if form.pasteMethod !== 'none' && form.pasteMethod !== 'direct'}
			<SettingsRow>
				<div>
					<label for="paste-delay" class="text-xs font-[570] text-ink">
						Clipboard paste delay
					</label>
					<p class="mt-[0.2rem] text-[0.6875rem] leading-[1.45] text-ink-muted">
						Wait for the clipboard owner before sending the shortcut.
					</p>
				</div>
				<div class="flex items-center gap-2">
					<Input
						class="w-24"
						id="paste-delay"
						type="number"
						min="0"
						max="1000"
						disabled={settings.locked}
						value={form.pasteDelay}
						oninput={(event) => form.setPasteDelay(event.currentTarget.value)}
					/>
					<span class="font-mono text-[0.6875rem] text-ink-muted">ms</span>
				</div>
			</SettingsRow>
		{/if}
	</SettingsPanel>
</SettingsSection>

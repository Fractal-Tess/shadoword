<script lang="ts">
	import { Send } from '@lucide/svelte';
	import BrutalistSelect from '$lib/components/BrutalistSelect.svelte';
	import { Input } from '$lib/components/ui/input';
	import { Switch } from '$lib/components/ui/switch';
	import { getSettingsContext } from '$lib/settings/context.svelte';
	import { PASTE_METHOD_OPTIONS } from '$lib/settings/options';

	const settings = getSettingsContext();
	const form = settings.form;
</script>

<section id="output">
	<header>
		<div class="section-icon"><Send size={16} /></div>
		<div>
			<h2 class="display-legend">Output</h2>
			<p>Control where completed transcript text is delivered.</p>
		</div>
	</header>
	<div class="setting-list">
		<div class="setting-row">
			<div>
				<label for="copy-final">Copy final transcript</label>
				<p>Write the completed transcript to the system clipboard.</p>
			</div>
			<Switch
				id="copy-final"
				checked={form.copyFinal}
				onCheckedChange={(checked) => form.setCopyFinal(checked)}
				disabled={settings.locked}
				aria-label="Copy final transcript"
			/>
		</div>
		<div class="setting-row">
			<div>
				<label for="paste-method">Active-window delivery</label>
				<p>Type directly or paste into the active window through the native host.</p>
			</div>
			<BrutalistSelect
				id="paste-method"
				value={form.pasteMethod}
				onValueChange={(value) => form.setPasteMethod(value)}
				options={PASTE_METHOD_OPTIONS}
				disabled={settings.locked}
				ariaLabel="Active-window delivery"
			/>
		</div>
		{#if form.pasteMethod !== 'none' && form.pasteMethod !== 'direct'}
			<div class="setting-row">
				<div>
					<label for="paste-delay">Clipboard paste delay</label>
					<p>Wait for the clipboard owner before sending the shortcut.</p>
				</div>
				<div class="delay-control">
					<Input
						id="paste-delay"
						type="number"
						min="0"
						max="1000"
						disabled={settings.locked}
						value={form.pasteDelay}
						oninput={(event) => form.setPasteDelay(event.currentTarget.value)}
					/>
					<span>ms</span>
				</div>
			</div>
		{/if}
	</div>
</section>

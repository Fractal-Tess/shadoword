<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Select } from '$lib/components/ui/select';
	import SettingsRow from '../SettingsRow.svelte';
	import { getSettingsContext } from '../_state/context.svelte';
	import { SHORTCUT_MODE_OPTIONS } from '../_state/options';

	const settings = getSettingsContext();
	const form = settings.form;
</script>

<SettingsRow>
	<div>
		<label for="shortcut-key" class="text-xs font-[570] text-ink">Global shortcut</label>
		<p class="mt-[0.2rem] text-[0.6875rem] leading-[1.45] text-ink-muted">
			Registered globally by the native desktop host when this shortcut changes.
		</p>
	</div>
	<Button
		id="shortcut-key"
		variant={form.shortcutCapturing ? 'destructive' : 'outline'}
		size="lg"
		class="w-[var(--control-width)] max-w-full justify-start font-mono text-[0.6875rem] font-[650]"
		disabled={settings.locked}
		onclick={() => form.toggleShortcutCapture()}
		aria-pressed={form.shortcutCapturing}
	>
		{form.shortcutCapturing ? 'Press shortcut…' : form.shortcut}
	</Button>
</SettingsRow>
{#if form.shortcutError || settings.app.hotkeyError}
	<SettingsRow class="grid-cols-1 text-[0.6875rem] text-scarlet-lamp" role="alert">
		<span>{form.shortcutError || settings.app.hotkeyError}</span>
	</SettingsRow>
{/if}
<SettingsRow>
	<div>
		<label for="shortcut-mode" class="text-xs font-[570] text-ink">Shortcut behavior</label>
		<p class="mt-[0.2rem] text-[0.6875rem] leading-[1.45] text-ink-muted">
			Hold to speak or press once to toggle capture.
		</p>
	</div>
	<Select
		id="shortcut-mode"
		class="w-[var(--control-width)] max-w-full"
		value={form.shortcutMode}
		onValueChange={(value) => form.setShortcutMode(value)}
		options={SHORTCUT_MODE_OPTIONS}
		disabled={settings.locked}
		ariaLabel="Shortcut behavior"
	/>
</SettingsRow>

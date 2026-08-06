<script lang="ts">
	import { AppWindow } from '@lucide/svelte';
	import { Switch } from '$lib/components/ui/switch';
	import SettingsPanel from '../SettingsPanel.svelte';
	import SettingsRow from '../SettingsRow.svelte';
	import SettingsSection from '../SettingsSection.svelte';
	import { getSettingsContext } from '../_state/context.svelte';

	const settings = getSettingsContext();
	const form = settings.form;
</script>

<SettingsSection id="application" title="Application" description="Window and background behavior.">
	{#snippet icon()}<AppWindow size={16} aria-hidden="true" />{/snippet}
	<SettingsPanel>
		<SettingsRow>
			<div>
				<label for="window-title-bar" class="text-xs font-[570] text-ink"> Window title bar </label>
				<p class="mt-[0.2rem] text-[0.6875rem] leading-[1.45] text-ink-muted">
					Show draggable window controls. Disable when your compositor already manages windows.
				</p>
			</div>
			<Switch
				id="window-title-bar"
				checked={form.showWindowTitleBar}
				onCheckedChange={(checked) => form.setShowWindowTitleBar(checked)}
				disabled={settings.locked}
				aria-label="Show window title bar"
			/>
		</SettingsRow>
		<SettingsRow>
			<div>
				<label for="close-tray" class="text-xs font-[570] text-ink">Close to tray</label>
				<p class="mt-[0.2rem] text-[0.6875rem] leading-[1.45] text-ink-muted">
					Hide the window on close while keeping hotkeys and the tray icon active.
				</p>
			</div>
			<Switch
				id="close-tray"
				checked={form.closeToTray}
				onCheckedChange={(checked) => form.setCloseToTray(checked)}
				disabled={settings.locked}
				aria-label="Close to tray"
			/>
		</SettingsRow>
	</SettingsPanel>
</SettingsSection>

<script lang="ts">
	import { AppWindow } from '@lucide/svelte';
	import { Switch } from '$lib/components/ui/switch';
	import { getSettingsContext } from '$lib/settings/context.svelte';

	const settings = getSettingsContext();
	const form = settings.form;
</script>

<section id="application">
	<header>
		<div class="section-icon"><AppWindow size={16} /></div>
		<div>
			<h2 class="display-legend">Application</h2>
			<p>Window and background behavior.</p>
		</div>
	</header>
	<div class="setting-list">
		<div class="setting-row">
			<div>
				<label for="close-tray">Close to tray</label>
				<p>Hide the window on close while keeping hotkeys and the tray icon active.</p>
			</div>
			<Switch
				id="close-tray"
				checked={form.closeToTray}
				onCheckedChange={(checked) => form.setCloseToTray(checked)}
				disabled={settings.locked}
				aria-label="Close to tray"
			/>
		</div>
	</div>
</section>

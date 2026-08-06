<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { getTranscribeContext, modeLabel } from './context';

	const context = getTranscribeContext();

	function handleAction() {
		if (context.mode === 'open_router' && !context.app.openRouterReady) {
			context.onOpenSettings();
		} else if (context.app.captureState === 'error') {
			context.app.clearError();
		} else {
			void context.app.refreshOverview();
		}
	}
</script>

<div
	class="absolute top-[1.15rem] right-5 grid max-w-68 justify-items-start gap-[0.4rem] border border-l-2 border-scarlet bg-plate px-[0.85rem] py-3 [@media(max-width:860px)]:static [@media(max-width:860px)]:mx-5 [@media(max-width:860px)]:max-w-none"
	role="alert"
>
	<strong class="font-mono text-[0.8125rem] leading-normal font-normal text-ink">
		{context.app.captureState === 'error'
			? 'The last capture failed'
			: `${modeLabel(context.mode)} transcription unavailable`}
	</strong>
	<span class="font-mono text-xs leading-[1.4] tracking-[0.04em] text-ink-muted">
		{context.app.error ??
			(context.mode === 'remote'
				? 'Check the endpoint and bearer token in Settings, then retry.'
				: context.mode === 'open_router'
					? context.app.openRouterCredentialState === 'invalid'
						? 'OpenRouter rejected the saved key. Replace it in Execution settings.'
						: context.app.openRouterCredentialState === 'checking'
							? 'Shadoword is checking the saved key with OpenRouter.'
							: 'Enter and verify an OpenRouter API key in Execution settings.'
					: 'Select or download a local model, then refresh the runtime.')}
	</span>
	<Button class="mt-[0.2rem]" variant="outline" size="sm" onclick={handleAction}>
		{context.mode === 'open_router' && !context.app.openRouterReady
			? context.app.settings?.openrouter_key_configured
				? 'Review key'
				: 'Configure key'
			: context.app.captureState === 'error'
				? 'Dismiss error'
				: 'Try again'}
	</Button>
</div>

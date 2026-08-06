<script lang="ts">
	import { Mic2, Square } from '@lucide/svelte';
	import { Button } from '$lib/components/ui/button';
	import { cn } from '$lib/utils';
	import { getTranscribeContext, modeLabel } from './context';

	const context = getTranscribeContext();

	const setRecording = async () => {
		context.setCopied(false);
		if (context.app.recording) await context.app.stopRecording();
		else await context.app.startRecording();
	};
</script>

<div
	class="relative grid content-start gap-[1.6rem] px-5 pt-[1.15rem] pb-6 [@media(max-width:860px)]:gap-[1.2rem] [@media(max-width:860px)]:pb-30"
>
	<div class="grid max-w-80 justify-items-start">
		<p
			id="capture-title"
			class="mt-0 mb-[0.85rem] font-mono text-xs leading-[1.3] font-medium tracking-[0.14em] text-ink-muted uppercase"
		>
			{context.app.recording
				? `${modeLabel(context.mode)} ${context.transcriptionMode}`
				: 'Capture'}
		</p>
		<button
			type="button"
			class={cn(
				'relative grid size-20 cursor-pointer place-items-center border-0 transition-[background-color,transform] duration-150 ease-linear active:not-disabled:translate-y-px disabled:cursor-not-allowed disabled:bg-raised disabled:text-ink-muted disabled:shadow-[inset_0_0_0_1px_var(--line-strong)] motion-reduce:transition-colors motion-reduce:active:not-disabled:translate-y-0',
				context.app.recording
					? 'bg-scarlet text-on-scarlet hover:bg-scarlet-deep'
					: 'bg-ink text-night hover:not-disabled:bg-on-scarlet'
			)}
			onclick={setRecording}
			aria-label={context.app.recording ? 'Stop recording' : 'Start recording'}
			aria-pressed={context.app.recording}
			disabled={!context.app.recording && (context.captureBlocked || context.app.processing)}
		>
			<span
				class={cn(
					'absolute inset-0 border border-transparent',
					context.app.recording &&
						'animate-[slab-pulse_1.8s_cubic-bezier(0.16,1,0.3,1)_infinite] border-scarlet motion-reduce:animate-none'
				)}
				aria-hidden="true"
			></span>
			{#if context.app.recording}
				<Square size={27} fill="currentColor" strokeWidth={0} aria-hidden="true" />
			{:else}
				<Mic2 size={32} strokeWidth={1.8} aria-hidden="true" />
			{/if}
		</button>
		<strong
			class="mt-[1.1rem] mr-[calc((var(--squeeze-label)-1)*100%)] origin-left [transform:scaleX(var(--squeeze-label))] font-display text-[1.375rem] leading-none font-normal tracking-[0.01em] text-ink uppercase"
		>
			{context.app.recording
				? 'Stop to finish'
				: context.app.processing
					? 'Transcribing'
					: context.captureBlocked
						? 'Unavailable'
						: 'Start recording'}
		</strong>
		<span class="mt-[0.35rem] font-mono text-xs leading-[1.4] tracking-[0.04em] text-ink-muted"
			>{context.app.recording
				? `${context.app.recordingSampleRate} Hz · mono · ${context.app.segmentCount} segments`
				: `${context.transcriptionMode === 'streaming' ? 'Pause-separated streaming' : 'Batch capture'} · native microphone`}</span
		>
		{#if context.app.recording}
			<Button
				class="mt-[0.7rem]"
				variant="ghost"
				size="sm"
				onclick={() => context.app.cancelRecording()}>Cancel</Button
			>
		{/if}
		<span class="sr-only" aria-live="polite">{context.surfaceTitle}</span>
	</div>

	<p
		class={cn(
			'm-0 min-h-[1.4rem] max-w-60 border-l border-transparent pl-[0.7rem] font-mono text-xs leading-[1.4] tracking-[0.04em] text-ink-dim [@media(max-width:860px)]:max-w-none',
			context.mode === 'local' && 'border-l-scarlet'
		)}
		aria-hidden={context.mode !== 'local'}
	>
		{#if context.mode === 'local'}One channel never lights up. Audio stays on this machine.{/if}
	</p>
</div>

<script lang="ts">
	import { Check, Copy, Trash2 } from '@lucide/svelte';
	import { Button } from '$lib/components/ui/button';
	import { Textarea } from '$lib/components/ui/textarea';
	import { getTranscribeContext, modeLabel } from './context';

	const context = getTranscribeContext();

	const copyTranscript = async () => {
		if (!context.app.transcript) return;
		await navigator.clipboard?.writeText(context.app.transcript);
		context.setCopied(true);
	};
</script>

<section
	class="grid min-h-0 grid-rows-[auto_minmax(0,1fr)_auto] overflow-hidden border border-line bg-plate"
	aria-labelledby="transcript-title"
>
	<header
		class="flex items-center justify-between gap-4 border-b border-line px-[0.9rem] py-[0.7rem]"
	>
		<h2
			id="transcript-title"
			class="font-display text-lg leading-none font-normal tracking-[0.035em] text-ink uppercase"
		>
			Working text
		</h2>
		<div class="flex gap-[0.35rem]">
			<Button
				variant="ghost"
				size="sm"
				onclick={() => (context.app.transcript = '')}
				disabled={!context.app.transcript}
			>
				<Trash2 size={13} />
				Clear
			</Button>
			<Button
				variant="outline"
				size="sm"
				onclick={copyTranscript}
				disabled={!context.app.transcript}
			>
				{#if context.copied}<Check size={13} />{:else}<Copy size={13} />{/if}
				{context.copied ? 'Copied' : 'Copy'}
			</Button>
		</div>
	</header>
	<Textarea
		bind:value={context.app.transcript}
		aria-label="Transcript text"
		placeholder="Your transcript will appear here…"
		class="min-h-0 resize-none rounded-none border-0 bg-night px-[0.9rem] py-[1.05rem] font-mono text-sm leading-[1.7] text-ink shadow-none placeholder:text-ink-muted focus-visible:border-0 focus-visible:ring-0 dark:bg-night"
	/>
	<footer
		class="flex gap-[1.15rem] border-t border-line px-[0.9rem] py-[0.6rem] font-mono text-xs leading-[1.4] tracking-[0.04em] text-ink-muted"
	>
		<span>{modeLabel(context.mode)} · {context.modelName}</span>
		<span>{context.app.history.length} session results</span>
		<span
			>{context.app.lastResult
				? `${context.app.lastResult.elapsed_ms} ms inference`
				: 'No inference yet'}</span
		>
	</footer>
</section>

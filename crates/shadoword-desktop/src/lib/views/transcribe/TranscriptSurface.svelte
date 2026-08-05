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

<section class="transcript-surface plate" aria-labelledby="transcript-title">
	<header>
		<h2 id="transcript-title" class="display-legend">Working text</h2>
		<div class="transcript-actions">
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
		class="transcript-editor"
	/>
	<footer class="mono-micro">
		<span>{modeLabel(context.mode)} · {context.modelName}</span>
		<span>{context.app.history.length} session results</span>
		<span
			>{context.app.lastResult
				? `${context.app.lastResult.elapsed_ms} ms inference`
				: 'No inference yet'}</span
		>
	</footer>
</section>

<style>
	.transcript-surface {
		display: grid;
		grid-template-rows: auto minmax(0, 1fr) auto;
		min-height: 0;
		overflow: hidden;
	}

	.transcript-surface header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		border-bottom: 1px solid var(--line);
		padding: 0.7rem 0.9rem;
	}

	.transcript-surface h2 {
		color: var(--ink);
	}

	.transcript-actions {
		display: flex;
		gap: 0.35rem;
	}

	.transcript-surface :global(.transcript-editor) {
		min-height: 0;
		resize: none;
		border: 0;
		border-radius: 0;
		padding: 1.05rem 0.9rem;
		background: var(--surface-0);
		color: var(--ink);
		font-family: var(--font-mono);
		font-size: 0.875rem;
		line-height: 1.7;
		box-shadow: none;
	}

	.transcript-surface :global(.transcript-editor)::placeholder {
		color: var(--ink-muted);
	}

	.transcript-surface footer {
		display: flex;
		gap: 1.15rem;
		border-top: 1px solid var(--line);
		padding: 0.6rem 0.9rem;
		color: var(--ink-muted);
	}
</style>

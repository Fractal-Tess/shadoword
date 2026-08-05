<script lang="ts">
	import { HardDrive } from '@lucide/svelte';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { getModelsContext } from './context';

	const context = getModelsContext();
</script>

<section class="custom-models" aria-labelledby="custom-title">
	<header>
		<div>
			<span>Custom models</span>
			<h2 id="custom-title" class="display-legend">Unverified files</h2>
		</div>
		<p>Files outside the checksum-verified catalog.</p>
	</header>
	{#if context.mode === 'local'}
		<div class="custom-row custom-path-row">
			<HardDrive size={17} />
			<div>
				<strong>Local model path</strong>
				<span>Use an existing Whisper GGML file outside the verified catalog.</span>
				<Input
					value={context.customPath}
					oninput={(event) => context.setCustomPath(event.currentTarget.value)}
					aria-label="Custom local model path"
				/>
			</div>
			<div class="custom-actions">
				<Button
					variant="outline"
					size="sm"
					disabled={context.controlsLocked ||
						!context.customPath.trim() ||
						context.customPath === context.runtime?.model_path}
					onclick={context.useCustomPath}
				>
					Use path
				</Button>
				<Button
					size="sm"
					disabled={context.controlsLocked}
					onclick={() => context.app.preloadLocalModel()}
				>
					Load / reload
				</Button>
			</div>
		</div>
	{:else}
		<div class="custom-row">
			<HardDrive size={17} />
			<div>
				<strong>Managed by the remote host</strong>
				<span>
					The current API exposes catalog selection and verified downloads, not arbitrary paths.
				</span>
			</div>
			<Badge variant="outline">API-managed</Badge>
		</div>
	{/if}
</section>

<style>
	.custom-models {
		border-top: 1px solid var(--line);
		padding-top: 1.25rem;
	}

	.custom-models > header {
		display: flex;
		align-items: end;
		justify-content: space-between;
		gap: 1rem;
		padding-bottom: 0.75rem;
	}

	.custom-models h2 {
		margin: 0.2rem 0 0;
		color: var(--ink);
	}

	.custom-models header span {
		color: var(--ink-muted);
		font-size: 0.6875rem;
		font-weight: 680;
		letter-spacing: 0.09em;
		text-transform: uppercase;
	}

	.custom-models header p {
		margin: 0;
		color: var(--ink-muted);
		font-size: 0.6875rem;
	}

	.custom-row {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr) auto auto;
		align-items: center;
		gap: 0.75rem;
		border: 1px solid var(--line);
		padding: 0.85rem 1rem;
		background: var(--surface-1);
		color: var(--ink-muted);
	}

	.custom-row > div {
		display: grid;
		gap: 0.25rem;
	}

	.custom-path-row > div:nth-child(2) {
		grid-template-columns: minmax(12rem, 1fr);
		gap: 0.55rem;
	}

	.custom-path-row :global(input) {
		font-family: var(--font-mono);
		font-size: 0.6875rem;
	}

	.custom-row > .custom-actions {
		display: flex;
		align-items: center;
		gap: 0.4rem;
	}

	.custom-row strong {
		color: var(--ink);
		font-size: 0.75rem;
	}

	.custom-row span {
		color: var(--ink-muted);
		font-family: var(--font-mono);
		font-size: 0.6875rem;
	}
</style>

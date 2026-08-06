<script lang="ts">
	import { HardDrive } from '@lucide/svelte';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { getModelsContext } from './context';

	const context = getModelsContext();
</script>

<section class="border-t border-line pt-5" aria-labelledby="custom-title">
	<header class="flex items-end justify-between gap-4 pb-3">
		<h2
			id="custom-title"
			class="m-0 font-display text-[1.125rem] leading-none font-normal tracking-[0.035em] text-ink uppercase"
		>
			Unverified files
		</h2>
		<p class="m-0 text-[0.6875rem] text-ink-muted">Files outside the checksum-verified catalog.</p>
	</header>
	{#if context.mode === 'local'}
		<div
			class="grid grid-cols-[auto_minmax(0,1fr)] items-center gap-3 border border-line bg-plate px-4 py-[0.85rem] text-ink-muted"
		>
			<HardDrive size={17} />
			<div class="grid gap-1">
				<strong class="text-xs text-ink">Local model path</strong>
				<span class="font-mono text-[0.6875rem] text-ink-muted">
					Use an existing Whisper GGML file outside the verified catalog.
				</span>
			</div>
			<div class="col-span-full">
				<Input
					class="font-mono text-[0.6875rem] md:text-[0.6875rem]"
					value={context.customPath}
					oninput={(event) => context.setCustomPath(event.currentTarget.value)}
					aria-label="Custom local model path"
				/>
			</div>
			<div class="col-span-full flex items-center justify-end gap-[0.4rem]">
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
		<div
			class="grid grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-3 border border-line bg-plate px-4 py-[0.85rem] text-ink-muted"
		>
			<HardDrive size={17} />
			<div class="grid gap-1">
				<strong class="text-xs text-ink">Managed by the remote host</strong>
				<span class="font-mono text-[0.6875rem] text-ink-muted">
					The current API exposes catalog selection and verified downloads, not arbitrary paths.
				</span>
			</div>
			<Badge variant="outline">API-managed</Badge>
		</div>
	{/if}
</section>
